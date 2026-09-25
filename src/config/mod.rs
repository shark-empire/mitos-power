//! Configuration: loads power.toml, profiles.toml, battery.toml,
//! sleep.toml, display.toml, thermal.toml from a directory (default
//! `/etc/mitos/power`, overridable with `--config-dir`). Each file is
//! independently optional, and so is every individual key within a file
//! that is present -- anything not specified falls back to the defaults
//! in `defaults.rs`.

mod defaults;
mod loader;
mod parser;
mod validation;

pub use defaults::{
    BatteryConfig, DisplayConfig, GeneralConfig, KeyboardConfig, LidConfig, PowerButtonConfig, ProfilesConfig, SecurityConfig,
    SleepConfig, ThermalConfig,
};

use crate::errors::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub security: SecurityConfig,
    pub power_button: PowerButtonConfig,
    pub keyboard: KeyboardConfig,
    pub profiles: ProfilesConfig,
    pub battery: BatteryConfig,
    pub sleep: SleepConfig,
    pub display: DisplayConfig,
    pub thermal: ThermalConfig,
}

impl Config {
    /// Loads and validates config from `dir`. See module docs for the
    /// per-file fallback behavior.
    pub fn load(dir: &Path) -> Result<Config> {
        loader::load(dir)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            security: SecurityConfig::default(),
            power_button: PowerButtonConfig::default(),
            keyboard: KeyboardConfig::default(),
            profiles: ProfilesConfig::default(),
            battery: BatteryConfig::default(),
            sleep: SleepConfig::default(),
            display: DisplayConfig::default(),
            thermal: ThermalConfig::default(),
        }
    }
}
