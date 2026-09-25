//! The hardware watcher loop: polls battery/thermal on a timer, ticks the
//! idle detector once a second, and routes udev hotplug events (lid,
//! power button, battery/AC uevents) to the right handler. Runs until the
//! shutdown signal fires.
//!
//! Simplification: battery and thermal share a single poll timer (ticking
//! at the shorter of `battery.poll_interval_secs` / `thermal.poll_interval_secs`)
//! rather than two independent timers -- re-reading battery sysfs a little
//! more often than strictly configured is harmless, and it keeps this loop
//! easy to reason about. Splitting them into independent `tokio::time::interval`
//! timers is a natural follow-up if that ever matters.

use crate::devices;
use crate::display::DisplayPowerState;
use crate::hardware::uevent;
use crate::idle;
use crate::manager::PowerManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

pub async fn run(manager: Arc<PowerManager>, mut shutdown_rx: broadcast::Receiver<()>) {
    let (battery_secs, thermal_secs) = {
        let config = manager.config.read().await;
        (config.battery.poll_interval_secs, config.thermal.poll_interval_secs)
    };
    let poll_secs = battery_secs.min(thermal_secs).max(1);

    let mut poll_timer = tokio::time::interval(Duration::from_secs(poll_secs));
    let mut idle_timer = tokio::time::interval(Duration::from_secs(1));

    let mut uevents = match uevent::spawn_monitor(&["power_supply"]) {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!("hardware event monitor unavailable, falling back to polling only: {e}");
            // An already-closed channel means `.recv()` in the select below
            // resolves to `None` immediately and that branch is effectively
            // disabled -- we still get everything through the poll timer.
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            rx
        }
    };

    tracing::info!("event loop started (poll every {poll_secs}s)");

    loop {
        tokio::select! {
            _ = poll_timer.tick() => {
                manager.refresh_and_run_policy().await;
                devices::lid::poll(&manager).await;
            }

            _ = idle_timer.tick() => {
                idle_tick(&manager).await;
            }

            Some(msg) = uevents.recv() => {
                route_uevent(&manager, msg).await;
            }

            _ = shutdown_rx.recv() => {
                tracing::info!("event loop shutting down");
                break;
            }
        }
    }
}

async fn idle_tick(manager: &Arc<PowerManager>) {
    let idle_inhibited = idle::inhibitors::is_idle_inhibited(&manager.inhibitors).await;

    let transition = { manager.idle.write().await.tick() };
    if let Some(new_tier) = transition {
        if idle_inhibited {
            tracing::debug!("idle tier would change to {new_tier:?}, but an Idle inhibitor is held -- not acting on it");
        } else {
            let idle_seconds = manager.idle.read().await.idle_seconds();
            manager.notify_idle_state_changed(idle_seconds, new_tier).await;

            let mut dp = manager.display_power.write().await;
            let target = match new_tier {
                crate::display::timeout::DisplayTier::Awake => DisplayPowerState::On,
                crate::display::timeout::DisplayTier::Dim => DisplayPowerState::Dimmed,
                crate::display::timeout::DisplayTier::Off => DisplayPowerState::Off,
            };
            dp.set_state(target);
        }
    }

    if !idle_inhibited && manager.idle.read().await.should_auto_suspend() {
        if let Err(e) = manager.suspend("idle-policy").await {
            tracing::debug!("idle-triggered suspend skipped: {e}");
        }
    }
}

async fn route_uevent(manager: &Arc<PowerManager>, msg: uevent::UeventMessage) {
    if msg.subsystem == "power_supply" {
        // Any power_supply add/change/remove is cheap to just fold into a
        // full refresh rather than trying to diff which attribute changed.
        manager.refresh_and_run_policy().await;
    }
}
