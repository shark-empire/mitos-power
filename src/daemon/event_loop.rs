//! The hardware watcher loop: polls battery/thermal on a timer, ticks the
//! idle detector once a second, routes udev hotplug events (battery/AC),
//! and routes live evdev input events (lid, power button, brightness
//! hotkeys) to the right handler. Runs until the shutdown signal fires.
//!
//! Simplification: battery and thermal share a single poll timer (ticking
//! at the shorter of `battery.poll_interval_secs` / `thermal.poll_interval_secs`)
//! rather than two independent timers -- re-reading battery sysfs a little
//! more often than strictly configured is harmless, and it keeps this loop
//! easy to reason about. Splitting them into independent `tokio::time::interval`
//! timers is a natural follow-up if that ever matters.

use crate::config::PowerButtonConfig;
use crate::devices;
use crate::display::DisplayPowerState;
use crate::hardware::evdev::{self, InputSignal};
use crate::hardware::uevent;
use crate::idle;
use crate::manager::PowerManager;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
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
            tracing::error!("udev hotplug monitor unavailable, falling back to polling only: {e}");
            // An already-closed channel means `.recv()` in the select below
            // resolves to `None` immediately and that branch is effectively
            // disabled -- we still get everything through the poll timer.
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            rx
        }
    };

    let mut input_signals = match evdev::spawn_watcher() {
        Ok(rx) => {
            tracing::info!("evdev input watcher active (lid/power-button/brightness-hotkeys are live)");
            rx
        }
        Err(e) => {
            tracing::warn!(
                "evdev input watcher unavailable ({e}); falling back to poll-based lid detection, \
                 power button and brightness hotkeys will not respond"
            );
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            rx
        }
    };
    let mut power_button = PowerButtonTracker::new();

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

            Some(signal) = input_signals.recv() => {
                route_input_signal(&manager, &mut power_button, signal).await;
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

async fn route_input_signal(manager: &Arc<PowerManager>, power_button: &mut PowerButtonTracker, signal: InputSignal) {
    match signal {
        InputSignal::LidSwitch(closed) => {
            devices::lid::handle_live_signal(manager, closed).await;
        }
        InputSignal::PowerButton(pressed) => {
            if pressed {
                power_button.on_press();
                return;
            }
            let config: PowerButtonConfig = manager.config.read().await.power_button.clone();
            let Some((held_for, recent_press_count)) = power_button.on_release(Duration::from_millis(config.multi_press_window_ms)) else {
                return; // release with no matching press we saw the start of (e.g. mid-press at daemon startup)
            };
            let action = devices::power_button::classify(held_for, recent_press_count, &config);
            devices::power_button::apply(manager, action).await;
        }
        InputSignal::BrightnessUp => devices::keyboard::handle_brightness_up(manager).await,
        InputSignal::BrightnessDown => devices::keyboard::handle_brightness_down(manager).await,
    }
}

/// Turns raw press/release evdev events into the `(held_for,
/// recent_press_count)` pair `devices::power_button::classify` expects.
/// Lives here (loop-local state) rather than in `devices::power_button`
/// itself, since it's specifically about bridging live hardware events --
/// `classify`/`apply` stay pure and independently testable without it.
struct PowerButtonTracker {
    press_start: Option<Instant>,
    recent_releases: VecDeque<Instant>,
}

impl PowerButtonTracker {
    fn new() -> Self {
        Self { press_start: None, recent_releases: VecDeque::new() }
    }

    fn on_press(&mut self) {
        self.press_start = Some(Instant::now());
    }

    /// Returns `(held_for, recent_press_count)` for this release, or
    /// `None` if we never saw the matching press (e.g. it started before
    /// the daemon did). `recent_press_count` includes this release and
    /// every other release within `window` of it.
    fn on_release(&mut self, window: Duration) -> Option<(Duration, u32)> {
        let start = self.press_start.take()?;
        let held_for = start.elapsed();

        let now = Instant::now();
        self.recent_releases.push_back(now);
        while let Some(&front) = self.recent_releases.front() {
            if now.duration_since(front) > window {
                self.recent_releases.pop_front();
            } else {
                break;
            }
        }
        Some((held_for, self.recent_releases.len() as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_without_a_press_is_ignored() {
        let mut tracker = PowerButtonTracker::new();
        assert!(tracker.on_release(Duration::from_millis(600)).is_none());
    }

    #[test]
    fn single_press_reports_count_one() {
        let mut tracker = PowerButtonTracker::new();
        tracker.on_press();
        let (_, count) = tracker.on_release(Duration::from_millis(600)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn two_rapid_presses_both_count_within_the_window() {
        let mut tracker = PowerButtonTracker::new();
        tracker.on_press();
        tracker.on_release(Duration::from_millis(600)).unwrap();
        tracker.on_press();
        let (_, count) = tracker.on_release(Duration::from_millis(600)).unwrap();
        assert_eq!(count, 2);
    }
}
