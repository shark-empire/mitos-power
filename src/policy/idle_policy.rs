//! Thin AC/battery-aware wrapper over `idle::policy::effective_timeouts`.
//! `idle::policy` computes timeouts from (display config, active profile);
//! this module is the place a future AC-vs-battery differentiation would
//! live (e.g. shorter off-timeout on battery even within the same
//! profile) without having to change `idle::detector` itself. Currently a
//! straight passthrough -- see docs/architecture.md "Policy engine".

use crate::config::DisplayConfig;
use crate::display::timeout::DisplayTimeouts;
use crate::profiles::ProfileInfo;

pub fn timeouts_for(display_config: &DisplayConfig, profile: &ProfileInfo, _on_ac: bool) -> DisplayTimeouts {
    crate::idle::policy::effective_timeouts(display_config, profile)
}
