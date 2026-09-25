//! Typed `params` payloads for methods that take arguments, and the
//! canonical list of method names. Methods that take no params (`Ping`,
//! `GetPowerState`, `ListProfiles`, ...) have no struct here -- the server
//! ignores `params` for those. Full per-method reference: docs/ipc.md.

use crate::inhibitor::{InhibitMode, InhibitWhat};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
pub struct GetBatteryParams {
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetProfileParams {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct SetBrightnessParams {
    pub percent: u8,
}

#[derive(Debug, Default, Deserialize)]
pub struct ForceParams {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleShutdownParams {
    pub at: DateTime<Utc>,
    #[serde(default)]
    pub reboot: bool,
}

#[derive(Debug, Deserialize)]
pub struct CancelScheduledParams {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AcquireInhibitorParams {
    pub who: String,
    pub why: String,
    pub what: InhibitWhat,
    #[serde(default = "default_mode")]
    pub mode: InhibitMode,
}

fn default_mode() -> InhibitMode {
    InhibitMode::Block
}

#[derive(Debug, Deserialize)]
pub struct ReleaseInhibitorParams {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct SetIdleTimeoutParams {
    pub secs: u64,
}

/// Every method name mitos-power's IPC server accepts, grouped by required
/// privilege. Kept as one source of truth so `permissions::required` and
/// docs/ipc.md's table can both be generated/checked against it.
pub mod method_names {
    pub const PUBLIC: &[&str] = &[
        "Ping",
        "GetVersion",
        "GetPowerState",
        "GetBattery",
        "GetBatteries",
        "GetAcState",
        "GetProfile",
        "ListProfiles",
        "GetBrightness",
        "GetThermalState",
        "GetThermalZones",
        "ListInhibitors",
        "GetLidState",
        "GetIdleState",
        "AcquireInhibitor",
        "ReleaseInhibitor",
        "ReportActivity",
    ];

    pub const SESSION: &[&str] = &["SetBrightness", "SetProfile", "SetIdleTimeout"];

    pub const PRIVILEGED: &[&str] = &[
        "Suspend",
        "Hibernate",
        "HybridSleep",
        "SuspendThenHibernate",
        "Shutdown",
        "Reboot",
        "Poweroff",
        "Logout",
        "ScheduleShutdown",
        "CancelScheduledOperation",
    ];
}
