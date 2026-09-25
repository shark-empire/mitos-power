//! Voltage helpers.

use super::device::BatteryDevice;

pub fn voltage_now_v(dev: &BatteryDevice) -> Option<f64> {
    dev.voltage_now_uv.map(|v| v as f64 / 1_000_000.0)
}
