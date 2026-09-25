//! Point-in-time diagnostic snapshot, for troubleshooting and a future
//! `GetDiagnostics` IPC method.

use crate::hardware::capabilities::HardwareCapabilities;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    pub uptime_secs: u64,
    pub version: &'static str,
    pub capabilities: HardwareCapabilities,
}

pub fn collect(uptime_secs: u64) -> Diagnostics {
    Diagnostics { uptime_secs, version: env!("CARGO_PKG_VERSION"), capabilities: HardwareCapabilities::detect() }
}
