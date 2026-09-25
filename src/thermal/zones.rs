//! Per-zone raw reads via /sys/class/thermal/thermal_zoneN.

use super::throttling;
use crate::errors::Result;
use crate::hardware::sysfs;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct ThermalZoneInfo {
    pub id: String,
    pub zone_type: String,
    #[serde(skip)]
    path: PathBuf,
    pub temperature_c: Option<f32>,
    pub trip_warning_c: Option<f32>,
    pub trip_critical_c: Option<f32>,
}

impl ThermalZoneInfo {
    pub fn refresh(&mut self) -> Result<()> {
        self.temperature_c = sysfs::read_i64(self.path.join("temp")).ok().map(|v| v as f32 / 1000.0);
        Ok(())
    }
}

pub fn discover() -> Result<Vec<ThermalZoneInfo>> {
    let mut out = Vec::new();
    for path in sysfs::list_matching("/sys/class/thermal", "thermal_zone")? {
        let id = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let zone_type = sysfs::read_trimmed_opt(path.join("type")).unwrap_or_else(|| "unknown".into());
        let (trip_warning_c, trip_critical_c) = throttling::read_trip_points(&path);
        let mut zone = ThermalZoneInfo { id, zone_type, path, temperature_c: None, trip_warning_c, trip_critical_c };
        zone.refresh()?;
        out.push(zone);
    }
    Ok(out)
}
