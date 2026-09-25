//! The aggregate, point-in-time view of the whole system returned by
//! `GetPowerState` -- the single call a new GUI/session component should
//! make first to paint an initial power indicator.

use crate::ac::AcState;
use crate::battery::BatteryInfo;
use crate::profiles::ProfileKind;
use crate::thermal::ThermalLevel;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PowerStateSnapshot {
    pub on_ac: bool,
    pub batteries: Vec<BatteryInfo>,
    pub overall_percentage: Option<f32>,
    pub profile: ProfileKind,
    pub lid_closed: Option<bool>,
    pub display_brightness_percent: Option<u8>,
    pub thermal_level: ThermalLevel,
    pub active_inhibitors: usize,
    pub idle_seconds: u64,
}
