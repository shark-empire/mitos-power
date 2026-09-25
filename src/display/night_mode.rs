//! Night-mode (color temperature) *state* tracking. Like display blanking,
//! the actual color-temperature shift happens in the compositor
//! (mitos-gui); mitos-power just owns the on/off + schedule state so it
//! survives daemon restarts and is queryable over IPC.

use chrono::NaiveTime;

#[derive(Debug, Clone)]
pub struct NightModeState {
    pub enabled: bool,
    pub start: Option<NaiveTime>,
    pub end: Option<NaiveTime>,
}

impl NightModeState {
    pub fn from_config(enabled: bool, start: &str, end: &str) -> Self {
        Self {
            enabled,
            start: NaiveTime::parse_from_str(start, "%H:%M").ok(),
            end: NaiveTime::parse_from_str(end, "%H:%M").ok(),
        }
    }

    /// Whether night mode should be active right now, given its schedule.
    /// Handles ranges that wrap past midnight (e.g. 21:00 -> 07:00).
    pub fn is_active_at(&self, now: NaiveTime) -> bool {
        if !self.enabled {
            return false;
        }
        match (self.start, self.end) {
            (Some(start), Some(end)) if start <= end => now >= start && now < end,
            (Some(start), Some(end)) => now >= start || now < end, // wraps midnight
            _ => false,
        }
    }
}
