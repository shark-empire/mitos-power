//! Custom, user-defined profiles loaded from profiles.toml's `[[profile]]`
//! entries (anything beyond the three built-ins).

use super::profile::{ProfileInfo, ProfileKind, ProfileSettings};
use serde::Deserialize;

/// Raw shape of one `[[profile]]` table in profiles.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct CustomProfileToml {
    pub name: String,
    pub cpu_governor: String,
    #[serde(default)]
    pub cpu_boost: bool,
    pub display_timeout_secs: u64,
    #[serde(default = "default_brightness")]
    pub display_max_brightness_percent: u8,
    #[serde(default)]
    pub background_throttle: bool,
    #[serde(default)]
    pub description: String,
}

fn default_brightness() -> u8 {
    100
}

impl From<CustomProfileToml> for ProfileInfo {
    fn from(raw: CustomProfileToml) -> Self {
        ProfileInfo {
            name: raw.name.clone(),
            kind: ProfileKind::Custom(raw.name),
            settings: ProfileSettings {
                cpu_governor: raw.cpu_governor,
                cpu_boost: raw.cpu_boost,
                display_timeout_secs: raw.display_timeout_secs,
                display_max_brightness_percent: raw.display_max_brightness_percent,
                background_throttle: raw.background_throttle,
                description: raw.description,
            },
            is_custom: true,
        }
    }
}
