//! Live keyboard/switch event monitoring via evdev -- the counterpart to
//! `uevent.rs` for individual key presses and switch state changes, which
//! (unlike hotplug) uevents don't carry.
//!
//! **Not part of the original spec's file list.** Added because
//! `devices::lid`/`power_button`/`keyboard` need a real event source
//! rather than polling (lid) or nothing at all (power button, brightness
//! hotkeys) -- see audit.md for the full history of that gap.
//!
//! Implementation note, in the same spirit as `uevent.rs`: this bridges a
//! blocking, C-library-backed API into tokio via one dedicated OS thread
//! per watched device, forwarding parsed events over an mpsc channel.
//! **The exact `evdev` crate method/type names used below (`enumerate`,
//! `supported_switches`, `supported_keys`, `fetch_events`, `SwitchType`,
//! `Key`, `InputEventKind`) match the shape of the 0.12-series API as I
//! recall it, written without the ability to check docs.rs or compile --
//! verify against whatever version actually resolves before trusting this
//! file.** This is the single riskiest file in the codebase from a "did I
//! get the API right" standpoint; everything else that touches an
//! external crate is either simpler (serde/tokio/thiserror) or has a
//! narrower surface (nix's reboot(2), udev's monitor socket).

use crate::errors::{PowerError, Result};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSignal {
    /// SW_LID switch value changed. `true` = closed.
    LidSwitch(bool),
    /// KEY_POWER transitioned. `true` on press, `false` on release --
    /// consumers time press duration themselves from the press/release
    /// pair (see `daemon::event_loop`'s `PowerButtonTracker`).
    PowerButton(bool),
    BrightnessUp,
    BrightnessDown,
}

/// Enumerates `/dev/input/event*`, picks out devices that report the
/// switches/keys mitos-power cares about, and spawns one reader thread per
/// matching device forwarding onto a shared channel.
///
/// Devices are discovered once, at startup, from this single call --
/// a keyboard/lid hotplugged in afterward won't be picked up without a
/// daemon restart. Watching udev "input" add events and re-scanning would
/// close that gap; not implemented (see audit.md).
pub fn spawn_watcher() -> Result<mpsc::Receiver<InputSignal>> {
    let (tx, rx) = mpsc::channel(32);
    let mut spawned_any = false;

    for (path, mut device) in evdev::enumerate() {
        let watches_lid = device.supported_switches().map(|s| s.contains(evdev::SwitchType::SW_LID)).unwrap_or(false);
        let watches_keys = device
            .supported_keys()
            .map(|k| {
                k.contains(evdev::Key::KEY_POWER) || k.contains(evdev::Key::KEY_BRIGHTNESSUP) || k.contains(evdev::Key::KEY_BRIGHTNESSDOWN)
            })
            .unwrap_or(false);

        if !watches_lid && !watches_keys {
            continue;
        }

        let tx = tx.clone();
        let path_for_log = path.clone();
        std::thread::spawn(move || {
            loop {
                let events = match device.fetch_events() {
                    Ok(events) => events,
                    Err(e) => {
                        tracing::warn!("evdev read error on {}, stopping watcher for this device: {e}", path_for_log.display());
                        return;
                    }
                };
                for ev in events {
                    let signal = match ev.kind() {
                        evdev::InputEventKind::Switch(evdev::SwitchType::SW_LID) => Some(InputSignal::LidSwitch(ev.value() != 0)),
                        evdev::InputEventKind::Key(evdev::Key::KEY_POWER) => Some(InputSignal::PowerButton(ev.value() != 0)),
                        // value == 1 is "pressed"; ignore release (0) and
                        // any auto-repeat (2) so each physical press steps
                        // brightness exactly once.
                        evdev::InputEventKind::Key(evdev::Key::KEY_BRIGHTNESSUP) if ev.value() == 1 => Some(InputSignal::BrightnessUp),
                        evdev::InputEventKind::Key(evdev::Key::KEY_BRIGHTNESSDOWN) if ev.value() == 1 => Some(InputSignal::BrightnessDown),
                        _ => None,
                    };
                    if let Some(signal) = signal {
                        if tx.blocking_send(signal).is_err() {
                            return; // receiver dropped -> daemon shutting down
                        }
                    }
                }
            }
        });
        spawned_any = true;
    }

    if !spawned_any {
        return Err(PowerError::Unsupported("no evdev device reports SW_LID or KEY_POWER/KEY_BRIGHTNESS*".into()));
    }

    Ok(rx)
}
