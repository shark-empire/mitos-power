//! Core profile types.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "name")]
pub enum ProfileKind {
    Performance,
    Balanced,
    PowerSaver,
    Custom(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileSettings {
    pub cpu_governor: String,
    pub cpu_boost: bool,
    pub display_timeout_secs: u64,
    pub display_max_brightness_percent: u8,
    pub background_throttle: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub name: String,
    pub kind: ProfileKind,
    pub settings: ProfileSettings,
    pub is_custom: bool,
}
