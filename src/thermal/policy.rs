//! Pure classification: highest zone temperature vs. configured thresholds.

use super::temperature;
use super::zones::ThermalZoneInfo;
use super::ThermalLevel;
use crate::config::ThermalConfig;

pub fn classify(zones: &[ThermalZoneInfo], config: &ThermalConfig) -> ThermalLevel {
    match temperature::highest_c(zones) {
        Some(t) if t >= config.critical_temp_c => ThermalLevel::Critical,
        Some(t) if t >= config.warning_temp_c => ThermalLevel::Warning,
        _ => ThermalLevel::Nominal,
    }
}
