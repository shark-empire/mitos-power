//! Idle tier transitions through the public API (also unit-tested inside
//! src/idle/detector.rs; this exercises the same behavior across the
//! crate's public boundary).

use mitos_power::config::DisplayConfig;
use mitos_power::display::timeout::DisplayTier;
use mitos_power::idle::IdleDetector;

fn zero_timeout_config() -> DisplayConfig {
    DisplayConfig {
        default_brightness_percent: 70,
        dim_timeout_secs: 0,
        off_timeout_secs: 0,
        suspend_on_idle: false,
        suspend_timeout_secs: 1800,
        night_mode_enabled: false,
        night_mode_start: "21:00".into(),
        night_mode_end: "07:00".into(),
    }
}

#[test]
fn idle_detector_transitions_to_off_immediately_with_zero_timeouts() {
    let mut detector = IdleDetector::new(&zero_timeout_config());
    assert_eq!(detector.tick(), Some(DisplayTier::Off));
    assert!(detector.reset());
    assert_eq!(detector.state().tier, DisplayTier::Awake);
}
