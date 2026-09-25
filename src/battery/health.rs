//! Battery wear/health estimation (full capacity vs. design capacity).

use super::device::BatteryDevice;

/// Health as a percentage of design capacity. `None` if this hardware
/// doesn't expose a design-capacity attribute at all.
pub fn health_percent(dev: &BatteryDevice) -> Option<f32> {
    if let (Some(full), Some(design)) = (dev.energy_full_uwh, dev.energy_full_design_uwh) {
        if design > 0 {
            return Some((full as f32 / design as f32) * 100.0);
        }
    }
    if let (Some(full), Some(design)) = (dev.charge_full_uah, dev.charge_full_design_uah) {
        if design > 0 {
            return Some((full as f32 / design as f32) * 100.0);
        }
    }
    None
}

/// Rough qualitative bucket for `mitos-powerctl battery` and GUI badges.
pub fn health_label(percent: Option<f32>) -> &'static str {
    match percent {
        Some(p) if p >= 90.0 => "excellent",
        Some(p) if p >= 75.0 => "good",
        Some(p) if p >= 50.0 => "fair",
        Some(_) => "poor",
        None => "unknown",
    }
}
