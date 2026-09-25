//! Display-specific idle timeout tiers (dim, then off), separate from the
//! system-wide idle/suspend policy in `idle::policy`. Reads its thresholds
//! from display.toml / the active profile.

use std::time::Duration;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayTier {
    Awake,
    Dim,
    Off,
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayTimeouts {
    pub dim_after: Duration,
    pub off_after: Duration,
}

impl DisplayTimeouts {
    pub fn classify(&self, idle_for: Duration) -> DisplayTier {
        if idle_for >= self.off_after {
            DisplayTier::Off
        } else if idle_for >= self.dim_after {
            DisplayTier::Dim
        } else {
            DisplayTier::Awake
        }
    }
}
