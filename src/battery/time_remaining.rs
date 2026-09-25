//! Estimated time-to-empty / time-to-full, derived from present energy and
//! current power draw. These are estimates: real remaining time depends on
//! load, which changes constantly, so treat them as approximate.

use super::charging::ChargingState;
use super::device::BatteryDevice;
use std::time::Duration;

pub fn time_to_empty(dev: &BatteryDevice) -> Option<Duration> {
    let state: ChargingState = dev.status_raw.as_str().into();
    if state != ChargingState::Discharging {
        return None;
    }
    let power_w = super::current::power_now_w(dev)?;
    if power_w <= 0.0 {
        return None;
    }
    let energy_wh = super::capacity::energy_now_wh(dev)?;
    Some(Duration::from_secs_f64(((energy_wh / power_w) * 3600.0).max(0.0)))
}

pub fn time_to_full(dev: &BatteryDevice) -> Option<Duration> {
    let state: ChargingState = dev.status_raw.as_str().into();
    if state != ChargingState::Charging {
        return None;
    }
    let power_w = super::current::power_now_w(dev)?;
    if power_w <= 0.0 {
        return None;
    }
    let now_wh = super::capacity::energy_now_wh(dev)?;
    let full_wh = super::capacity::energy_full_wh(dev)?;
    let remaining_wh = (full_wh - now_wh).max(0.0);
    Some(Duration::from_secs_f64(((remaining_wh / power_w) * 3600.0).max(0.0)))
}
