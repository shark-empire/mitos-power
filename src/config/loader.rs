//! Reads each of the six config files from the config directory, falling
//! back to built-in defaults for any file that's missing entirely (first
//! run / minimal install). A file that exists but fails to *parse* is
//! still a hard error -- silently ignoring a typo'd config is worse than
//! refusing to start with a clear message.

use super::defaults::PowerToml;
use super::parser::parse;
use super::{BatteryConfig, Config, DisplayConfig, ProfilesConfig, SleepConfig, ThermalConfig};
use crate::errors::Result;
use std::path::Path;

fn read_or_default<T: Default + serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(_) => Ok(T::default()),
    }
}

pub fn load(dir: &Path) -> Result<Config> {
    let power: PowerToml = read_or_default(&dir.join("power.toml"))?;
    let profiles: ProfilesConfig = read_or_default(&dir.join("profiles.toml"))?;
    let battery: BatteryConfig = read_or_default(&dir.join("battery.toml"))?;
    let sleep: SleepConfig = read_or_default(&dir.join("sleep.toml"))?;
    let display: DisplayConfig = read_or_default(&dir.join("display.toml"))?;
    let thermal: ThermalConfig = read_or_default(&dir.join("thermal.toml"))?;

    let config = Config {
        general: power.general,
        security: power.security,
        power_button: power.power_button,
        keyboard: power.keyboard,
        profiles,
        battery,
        sleep,
        display,
        thermal,
    };

    super::validation::validate(&config)?;
    Ok(config)
}
