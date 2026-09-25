//! Combines display.toml defaults with the *active profile's*
//! `display_timeout_secs` to produce the effective dim/off timeouts.
//! Called by `PowerManager::set_profile` on every profile switch -- this
//! is the concrete mechanism behind the spec's "policy-based, not
//! hardcoded" requirement for per-profile idle behavior.

use crate::config::DisplayConfig;
use crate::display::timeout::DisplayTimeouts;
use crate::profiles::ProfileInfo;
use std::time::Duration;

pub fn effective_timeouts(display_config: &DisplayConfig, profile: &ProfileInfo) -> DisplayTimeouts {
    // The profile's display_timeout_secs is the "go to off" threshold; dim
    // always fires at the configured dim_timeout_secs, or sooner if the
    // profile's off-timeout is shorter than that -- "off" should always
    // have a dim warning before it, whichever profile is active.
    let off_after = Duration::from_secs(profile.settings.display_timeout_secs);
    let configured_dim = Duration::from_secs(display_config.dim_timeout_secs);
    let dim_after = configured_dim.min(off_after);
    DisplayTimeouts { dim_after, off_after }
}
