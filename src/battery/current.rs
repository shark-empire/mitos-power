//! Current-draw / instantaneous power helpers.

use super::device::BatteryDevice;

pub fn current_now_a(dev: &BatteryDevice) -> Option<f64> {
    dev.current_now_ua.map(|v| v as f64 / 1_000_000.0)
}

/// Instantaneous power draw in watts. Prefers the kernel's own `power_now`
/// when present; falls back to computing volts * amps for drivers that only
/// expose current and voltage separately.
pub fn power_now_w(dev: &BatteryDevice) -> Option<f64> {
    if let Some(p) = dev.power_now_uw {
        return Some(p as f64 / 1_000_000.0);
    }
    match (current_now_a(dev), super::voltage::voltage_now_v(dev)) {
        (Some(i), Some(v)) => Some(i * v),
        _ => None,
    }
}
