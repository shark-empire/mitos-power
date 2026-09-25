//! Hotplug / hardware event monitoring via udev.
//!
//! mitos-power subscribes to the `power_supply`, `input` and `thermal`
//! kernel subsystems so it can react to battery/AC changes, lid-switch and
//! power-button events without polling sysfs on a tight timer.
//!
//! Implementation note: libudev's monitor socket is a blocking fd. Rather
//! than trying to bridge it into tokio's async-fd machinery (whose exact
//! API depends on the resolved `udev` crate version), we run the blocking
//! iterator on a dedicated OS thread and forward events over an mpsc
//! channel -- a standard, low-risk way to bring a blocking C library into
//! an async runtime.

use crate::errors::{PowerError, Result};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct UeventMessage {
    pub subsystem: String,
    pub action: String,
    pub sysname: String,
    pub devpath: String,
}

/// Spawn a background thread that forwards udev events for the given
/// subsystems onto an mpsc channel, and return the receiving end.
pub fn spawn_monitor(subsystems: &[&str]) -> Result<mpsc::Receiver<UeventMessage>> {
    let (tx, rx) = mpsc::channel(64);

    let mut builder = udev::MonitorBuilder::new()
        .map_err(|e| PowerError::Hardware(format!("udev monitor init: {e}")))?;
    for subsystem in subsystems {
        builder = builder
            .match_subsystem(subsystem)
            .map_err(|e| PowerError::Hardware(format!("udev match_subsystem({subsystem}): {e}")))?;
    }
    let socket = builder
        .listen()
        .map_err(|e| PowerError::Hardware(format!("udev listen: {e}")))?;

    std::thread::spawn(move || {
        for event in socket.iter() {
            let msg = UeventMessage {
                subsystem: event.subsystem().to_string_lossy().to_string(),
                action: event
                    .action()
                    .map(|a| a.to_string_lossy().to_string())
                    .unwrap_or_default(),
                sysname: event.sysname().to_string_lossy().to_string(),
                devpath: event.devpath().to_string_lossy().to_string(),
            };
            if tx.blocking_send(msg).is_err() {
                break; // receiver dropped -> daemon is shutting down
            }
        }
    });

    Ok(rx)
}
