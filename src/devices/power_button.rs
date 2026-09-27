//! Power-button press classification (short / long / multi-press).
//!
//! `classify()` is pure and unit tested; `apply()` carries out whatever it
//! returns. The live event source is `hardware::evdev::spawn_watcher`,
//! consumed by `daemon::event_loop`'s `PowerButtonTracker`, which turns
//! raw press/release timestamps into the `held_for`/`recent_press_count`
//! this module's `classify()` expects. Requires evdev to find a device
//! that reports `KEY_POWER` at startup -- see audit.md for what happens
//! when it doesn't (falls back to no power-button handling at all, same
//! as before this was wired up).

use crate::config::PowerButtonConfig;
use crate::manager::PowerManager;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    Ignore,
    Lock,
    Suspend,
    Hibernate,
    Shutdown,
    Reboot,
}

impl ButtonAction {
    fn from_config_str(s: &str) -> ButtonAction {
        match s {
            "lock" => ButtonAction::Lock,
            "suspend" => ButtonAction::Suspend,
            "hibernate" => ButtonAction::Hibernate,
            "shutdown" => ButtonAction::Shutdown,
            "reboot" => ButtonAction::Reboot,
            _ => ButtonAction::Ignore,
        }
    }
}

/// Given how long a press was held and how many presses have landed within
/// the configured multi-press window, decide which configured action (if
/// any) applies. Multi-press takes priority over the single-press
/// short/long distinction, matching how most laptops' physical power
/// buttons behave (a rapid double-press is its own gesture).
pub fn classify(held_for: Duration, recent_press_count: u32, config: &PowerButtonConfig) -> ButtonAction {
    if recent_press_count >= 2 {
        return ButtonAction::from_config_str(&config.multi_press);
    }
    if held_for >= Duration::from_millis(config.long_press_threshold_ms) {
        ButtonAction::from_config_str(&config.long_press)
    } else {
        ButtonAction::from_config_str(&config.short_press)
    }
}

pub async fn apply(manager: &Arc<PowerManager>, action: ButtonAction) {
    manager.notify_power_button_pressed(action).await;
    match action {
        ButtonAction::Ignore => {}
        ButtonAction::Lock => tracing::info!("power button: lock (delegated to mitos-session)"),
        ButtonAction::Suspend => {
            if let Err(e) = manager.suspend("power-button").await {
                tracing::debug!("power-button-triggered suspend skipped: {e}");
            }
        }
        ButtonAction::Hibernate => {
            if let Err(e) = manager.hibernate("power-button").await {
                tracing::debug!("power-button-triggered hibernate skipped: {e}");
            }
        }
        ButtonAction::Shutdown => {
            if let Err(e) = manager.shutdown("power-button", false).await {
                tracing::debug!("power-button-triggered shutdown skipped: {e}");
            }
        }
        ButtonAction::Reboot => {
            if let Err(e) = manager.reboot("power-button", false).await {
                tracing::debug!("power-button-triggered reboot skipped: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PowerButtonConfig {
        PowerButtonConfig {
            short_press: "suspend".into(),
            long_press: "shutdown".into(),
            multi_press: "ignore".into(),
            long_press_threshold_ms: 1500,
            multi_press_window_ms: 600,
        }
    }

    #[test]
    fn quick_tap_is_short_press() {
        assert_eq!(classify(Duration::from_millis(200), 1, &config()), ButtonAction::Suspend);
    }

    #[test]
    fn held_past_threshold_is_long_press() {
        assert_eq!(classify(Duration::from_millis(2000), 1, &config()), ButtonAction::Shutdown);
    }

    #[test]
    fn multiple_presses_take_priority_over_duration() {
        // Even a long hold counts as multi-press once recent_press_count >= 2.
        assert_eq!(classify(Duration::from_millis(2000), 2, &config()), ButtonAction::Ignore);
    }
}
