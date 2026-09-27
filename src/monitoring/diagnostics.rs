//! Point-in-time diagnostic snapshot, for troubleshooting and the
//! `GetDiagnostics` IPC method.

use super::statistics::MetricsSnapshot;
use crate::hardware::capabilities::HardwareCapabilities;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    pub uptime_secs: u64,
    pub version: &'static str,
    pub capabilities: HardwareCapabilities,
    pub metrics: MetricsSnapshot,
}

pub fn collect(uptime_secs: u64, metrics: MetricsSnapshot) -> Diagnostics {
    Diagnostics { uptime_secs, version: env!("CARGO_PKG_VERSION"), capabilities: HardwareCapabilities::detect(), metrics }
}
