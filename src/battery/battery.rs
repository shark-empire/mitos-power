//! Public battery snapshot type (what goes out over IPC) and the manager
//! that keeps every discovered battery refreshed.

use super::capacity;
use super::charging::ChargingState;
use super::device::BatteryDevice;
use super::health;
use super::statistics::BatteryStats;
use super::{current, temperature, time_remaining, voltage};
use crate::errors::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct BatteryInfo {
    pub id: String,
    pub present: bool,
    pub percentage: Option<f32>,
    pub status: ChargingState,
    pub energy_now_wh: Option<f64>,
    pub energy_full_wh: Option<f64>,
    pub power_now_w: Option<f64>,
    pub voltage_now_v: Option<f64>,
    pub current_now_a: Option<f64>,
    pub temperature_c: Option<f32>,
    pub cycle_count: Option<u64>,
    pub health_percent: Option<f32>,
    pub health_label: &'static str,
    pub time_to_empty_secs: Option<u64>,
    pub time_to_full_secs: Option<u64>,
    pub technology: Option<String>,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
}

impl From<&BatteryDevice> for BatteryInfo {
    fn from(dev: &BatteryDevice) -> Self {
        let health_percent = health::health_percent(dev);
        BatteryInfo {
            id: dev.id.clone(),
            present: dev.present,
            percentage: capacity::percentage(dev),
            status: dev.status_raw.as_str().into(),
            energy_now_wh: capacity::energy_now_wh(dev),
            energy_full_wh: capacity::energy_full_wh(dev),
            power_now_w: current::power_now_w(dev),
            voltage_now_v: voltage::voltage_now_v(dev),
            current_now_a: current::current_now_a(dev),
            temperature_c: temperature::temperature_c(dev),
            cycle_count: dev.cycle_count,
            health_percent,
            health_label: health::health_label(health_percent),
            time_to_empty_secs: time_remaining::time_to_empty(dev).map(|d: Duration| d.as_secs()),
            time_to_full_secs: time_remaining::time_to_full(dev).map(|d: Duration| d.as_secs()),
            technology: dev.technology.clone(),
            manufacturer: dev.manufacturer.clone(),
            model_name: dev.model_name.clone(),
        }
    }
}

/// What changed on the last `refresh()`, so the daemon's event loop can
/// decide whether to emit `BatteryChanged`, `ChargingChanged`, both, or
/// neither (avoids spamming subscribers on every poll tick).
#[derive(Debug, Clone, Copy, Default)]
pub struct RefreshResult {
    pub percentage_changed: bool,
    pub status_changed: bool,
}

impl RefreshResult {
    pub fn any(&self) -> bool {
        self.percentage_changed || self.status_changed
    }
}

pub struct BatteryManager {
    devices: Vec<BatteryDevice>,
    stats: HashMap<String, BatteryStats>,
}

impl BatteryManager {
    pub fn new() -> Result<Self> {
        let devices = BatteryDevice::discover()?;
        let stats = devices.iter().map(|d| (d.id.clone(), BatteryStats::default())).collect();
        Ok(Self { devices, stats })
    }

    /// Re-read every battery from sysfs, update rolling statistics, and
    /// report what changed.
    pub fn refresh(&mut self) -> Result<RefreshResult> {
        let mut result = RefreshResult::default();
        for dev in &mut self.devices {
            let before_pct = dev.capacity_percent;
            let before_status = dev.status_raw.clone();
            dev.refresh()?;
            if dev.capacity_percent != before_pct {
                result.percentage_changed = true;
            }
            if dev.status_raw != before_status {
                result.status_changed = true;
            }
            if let Some(pct) = capacity::percentage(dev) {
                let power = current::power_now_w(dev);
                self.stats.entry(dev.id.clone()).or_default().push(pct, power);
            }
        }
        Ok(result)
    }

    pub fn all(&self) -> Vec<BatteryInfo> {
        self.devices.iter().map(BatteryInfo::from).collect()
    }

    pub fn get(&self, id: &str) -> Option<BatteryInfo> {
        self.devices.iter().find(|d| d.id == id).map(BatteryInfo::from)
    }

    /// Simple mean across all present batteries. Good enough for a headline
    /// number; per-battery detail is always available via `all()`.
    pub fn overall_percentage(&self) -> Option<f32> {
        let values: Vec<f32> = self.all().iter().filter_map(|b| b.percentage).collect();
        if values.is_empty() {
            return None;
        }
        Some(values.iter().sum::<f32>() / values.len() as f32)
    }

    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    pub fn average_power_w(&self, id: &str) -> Option<f64> {
        self.stats.get(id)?.average_power_w()
    }
}
