//! Thermal classification against a machine with no thermal zones (this
//! sandbox and most CI containers).

use mitos_power::config::ThermalConfig;
use mitos_power::thermal::{ThermalLevel, ThermalManager};

#[test]
fn thermal_manager_construction_never_fails_even_with_zero_zones() {
    let config = ThermalConfig { poll_interval_secs: 5, warning_temp_c: 80.0, critical_temp_c: 95.0, critical_action: "emergency_shutdown".into() };
    let manager = ThermalManager::new(&config).expect("ThermalManager::new should not fail even with zero thermal zones present");
    // Before any refresh(), level defaults to Nominal.
    assert_eq!(manager.level(), ThermalLevel::Nominal);
}
