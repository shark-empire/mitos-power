//! Battery temperature. The kernel reports in tenths of a degree Celsius.

use super::device::BatteryDevice;

pub fn temperature_c(dev: &BatteryDevice) -> Option<f32> {
    dev.temp_decidegrees.map(|t| t as f32 / 10.0)
}
