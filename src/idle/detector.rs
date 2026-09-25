//! Ties `timeout::ActivityTimer` (raw clock) and `display::timeout::DisplayTimeouts`
//! (tier thresholds) together into the stateful detector `PowerManager` holds.

use super::timeout::ActivityTimer;
use crate::config::DisplayConfig;
use crate::display::timeout::{DisplayTier, DisplayTimeouts};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct IdleState {
    pub idle_seconds: u64,
    pub tier: DisplayTier,
}

pub struct IdleDetector {
    timer: ActivityTimer,
    timeouts: DisplayTimeouts,
    suspend_on_idle: bool,
    suspend_after: Duration,
    current_tier: DisplayTier,
}

impl IdleDetector {
    pub fn new(config: &DisplayConfig) -> Self {
        Self {
            timer: ActivityTimer::new(),
            timeouts: DisplayTimeouts {
                dim_after: Duration::from_secs(config.dim_timeout_secs),
                off_after: Duration::from_secs(config.off_timeout_secs),
            },
            suspend_on_idle: config.suspend_on_idle,
            suspend_after: Duration::from_secs(config.suspend_timeout_secs),
            current_tier: DisplayTier::Awake,
        }
    }

    /// Resets the activity clock. Returns `true` if this actually pulled
    /// the tier back from Dim/Off to Awake (i.e. is worth telling
    /// subscribers and the display about), `false` if we were already
    /// Awake and this is a no-op.
    pub fn reset(&mut self) -> bool {
        self.timer.reset();
        let was_awake = self.current_tier == DisplayTier::Awake;
        self.current_tier = DisplayTier::Awake;
        !was_awake
    }

    pub fn idle_seconds(&self) -> u64 {
        self.timer.elapsed().as_secs()
    }

    pub fn state(&self) -> IdleState {
        IdleState { idle_seconds: self.idle_seconds(), tier: self.current_tier }
    }

    pub fn set_off_timeout(&mut self, secs: u64) {
        self.timeouts.off_after = Duration::from_secs(secs);
    }

    pub fn set_timeouts(&mut self, timeouts: DisplayTimeouts) {
        self.timeouts = timeouts;
    }

    /// Re-evaluates the current tier against elapsed time. Returns
    /// `Some(new_tier)` only on an actual transition, so callers can tell
    /// "still off" from "just went off" without tracking that themselves.
    pub fn tick(&mut self) -> Option<DisplayTier> {
        let new_tier = self.timeouts.classify(self.timer.elapsed());
        if new_tier != self.current_tier {
            self.current_tier = new_tier;
            Some(new_tier)
        } else {
            None
        }
    }

    pub fn should_auto_suspend(&self) -> bool {
        self.suspend_on_idle && self.timer.elapsed() >= self.suspend_after
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(dim: u64, off: u64) -> DisplayConfig {
        DisplayConfig {
            default_brightness_percent: 70,
            dim_timeout_secs: dim,
            off_timeout_secs: off,
            suspend_on_idle: false,
            suspend_timeout_secs: 1800,
            night_mode_enabled: false,
            night_mode_start: "21:00".into(),
            night_mode_end: "07:00".into(),
        }
    }

    #[test]
    fn starts_awake_and_reset_is_a_noop() {
        let mut d = IdleDetector::new(&config(300, 600));
        assert_eq!(d.state().tier, DisplayTier::Awake);
        assert!(!d.reset()); // already awake -> no transition
    }

    #[test]
    fn tick_with_zero_timeouts_immediately_reports_off() {
        // 0s timeouts means "instantly idle", a clean way to exercise the
        // transition logic without sleeping in a unit test.
        let mut d = IdleDetector::new(&config(0, 0));
        assert_eq!(d.tick(), Some(DisplayTier::Off));
        // Second tick with no state change in between -> no further transition.
        assert_eq!(d.tick(), None);
    }

    #[test]
    fn reset_after_going_idle_reports_a_transition() {
        let mut d = IdleDetector::new(&config(0, 0));
        d.tick();
        assert_eq!(d.state().tier, DisplayTier::Off);
        assert!(d.reset());
        assert_eq!(d.state().tier, DisplayTier::Awake);
    }
}
