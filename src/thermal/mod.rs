//! Thermal monitoring. mitos-power only *observes* and reacts at the
//! policy level (dropping to powersave on Warning, emergency-shutdown on
//! Critical) -- it never writes kernel trip points and never tries to
//! out-throttle the kernel's own thermal driver, which remains responsible
//! for fundamental hardware safety regardless of what mitos-power does.

pub mod policy;
pub mod protection;
pub mod temperature;
pub mod throttling;
pub mod zones;

use crate::config::ThermalConfig;
use crate::errors::Result;
pub use zones::ThermalZoneInfo;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalLevel {
    Nominal,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThermalSummary {
    pub level: ThermalLevel,
    pub highest_temp_c: Option<f32>,
    pub warning_temp_c: f32,
    pub critical_temp_c: f32,
}

pub struct ThermalManager {
    zones: Vec<ThermalZoneInfo>,
    config: ThermalConfig,
    level: ThermalLevel,
}

impl ThermalManager {
    pub fn new(config: &ThermalConfig) -> Result<Self> {
        let zones = zones::discover()?;
        Ok(Self { zones, config: config.clone(), level: ThermalLevel::Nominal })
    }

    pub fn refresh(&mut self) -> Result<ThermalLevel> {
        for zone in &mut self.zones {
            zone.refresh()?;
        }
        self.level = policy::classify(&self.zones, &self.config);
        Ok(self.level)
    }

    pub fn zones(&self) -> Vec<ThermalZoneInfo> {
        self.zones.clone()
    }

    pub fn level(&self) -> ThermalLevel {
        self.level
    }

    pub fn summary(&self) -> ThermalSummary {
        ThermalSummary {
            level: self.level,
            highest_temp_c: temperature::highest_c(&self.zones),
            warning_temp_c: self.config.warning_temp_c,
            critical_temp_c: self.config.critical_temp_c,
        }
    }
}
