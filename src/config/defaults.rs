//! Every config struct, each with `#[serde(default)]` (so a partially
//! filled-in TOML file only overrides the keys it mentions) paired with an
//! explicit `Default` impl (derived `Default` would give empty strings for
//! things like `short_press`, which isn't a sane default).

use crate::profiles::custom::CustomProfileToml;
use serde::{Deserialize, Serialize};

// ---- power.toml ------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub socket_path: String,
    pub socket_group: String,
    pub log_level: String,
    pub state_dir: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            socket_path: "/run/mitos/power.sock".into(),
            socket_group: "power".into(),
            log_level: "info".into(),
            state_dir: "/var/lib/mitos/power".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub privileged_group: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self { privileged_group: "power".into() }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PowerButtonConfig {
    pub short_press: String,
    pub long_press: String,
    pub multi_press: String,
    pub long_press_threshold_ms: u64,
    pub multi_press_window_ms: u64,
}

impl Default for PowerButtonConfig {
    fn default() -> Self {
        Self {
            short_press: "suspend".into(),
            long_press: "shutdown".into(),
            multi_press: "ignore".into(),
            long_press_threshold_ms: 1500,
            multi_press_window_ms: 600,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct KeyboardConfig {
    pub brightness_hotkeys: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub(super) struct PowerToml {
    pub general: GeneralConfig,
    pub security: SecurityConfig,
    pub power_button: PowerButtonConfig,
    pub keyboard: KeyboardConfig,
}

// ---- profiles.toml -----------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProfilesConfig {
    pub default_profile: String,
    pub profile: Vec<CustomProfileToml>,
}

impl Default for ProfilesConfig {
    fn default() -> Self {
        Self { default_profile: "balanced".into(), profile: Vec::new() }
    }
}

// ---- battery.toml ------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BatteryConfig {
    pub poll_interval_secs: u64,
    pub low_threshold_percent: u8,
    pub low_action: String,
    pub critical_threshold_percent: u8,
    pub critical_action: String,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 30,
            low_threshold_percent: 20,
            low_action: "notify".into(),
            critical_threshold_percent: 5,
            critical_action: "hibernate".into(),
        }
    }
}

// ---- sleep.toml --------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LidConfig {
    pub close_action_ac: String,
    pub close_action_battery: String,
    pub open_action: String,
    pub external_monitor_ignore_lid: bool,
}

impl Default for LidConfig {
    fn default() -> Self {
        Self {
            close_action_ac: "lock".into(),
            close_action_battery: "suspend".into(),
            open_action: "wake".into(),
            external_monitor_ignore_lid: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SleepConfig {
    pub default_method: String,
    pub lock_before_suspend: bool,
    pub suspend_then_hibernate_delay_mins: u64,
    pub lid: LidConfig,
}

impl Default for SleepConfig {
    fn default() -> Self {
        Self {
            default_method: "suspend".into(),
            lock_before_suspend: true,
            suspend_then_hibernate_delay_mins: 120,
            lid: LidConfig::default(),
        }
    }
}

// ---- display.toml -------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub default_brightness_percent: u8,
    pub dim_timeout_secs: u64,
    pub off_timeout_secs: u64,
    pub suspend_on_idle: bool,
    pub suspend_timeout_secs: u64,
    pub night_mode_enabled: bool,
    pub night_mode_start: String,
    pub night_mode_end: String,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            default_brightness_percent: 70,
            dim_timeout_secs: 300,
            off_timeout_secs: 600,
            suspend_on_idle: false,
            suspend_timeout_secs: 1800,
            night_mode_enabled: false,
            night_mode_start: "21:00".into(),
            night_mode_end: "07:00".into(),
        }
    }
}

// ---- thermal.toml ------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ThermalConfig {
    pub poll_interval_secs: u64,
    pub warning_temp_c: f32,
    pub critical_temp_c: f32,
    /// Parsed and exposed for forward compatibility, but intentionally
    /// **not** consulted by `thermal::protection` today -- the Critical
    /// response is always a clean emergency shutdown regardless of this
    /// value, so a misconfigured thermal.toml can't silently disable the
    /// safety net. See audit.md.
    pub critical_action: String,
}

impl Default for ThermalConfig {
    fn default() -> Self {
        Self { poll_interval_secs: 5, warning_temp_c: 80.0, critical_temp_c: 95.0, critical_action: "emergency_shutdown".into() }
    }
}
