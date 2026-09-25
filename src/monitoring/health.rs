//! Daemon uptime/version self-report.

use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    pub uptime_secs: u64,
    pub version: &'static str,
}

pub struct HealthTracker {
    started_at: Instant,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self { started_at: Instant::now() }
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn report(&self) -> HealthReport {
        HealthReport { uptime_secs: self.uptime().as_secs(), version: env!("CARGO_PKG_VERSION") }
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new()
    }
}
