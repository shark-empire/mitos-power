//! Integration tests for the battery subsystem through the public API.
//! Deliberately environment-agnostic: these assertions hold whether
//! `cargo test` runs on a real laptop (with a real battery) or in a
//! container/CI with none.

use mitos_power::battery::BatteryManager;

#[test]
fn battery_manager_is_internally_consistent() {
    let manager = BatteryManager::new().expect("BatteryManager::new should not fail even with zero batteries present");
    let all = manager.all();
    assert_eq!(manager.is_empty(), all.is_empty());

    if all.is_empty() {
        assert!(manager.overall_percentage().is_none());
    }
    for battery in &all {
        if let Some(pct) = battery.percentage {
            assert!((0.0..=100.0).contains(&pct), "battery percentage {pct} out of range for {}", battery.id);
        }
    }
}
