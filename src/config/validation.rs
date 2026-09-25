//! Sanity checks on a loaded config, run once by `loader::load` right
//! after assembly. A config that fails validation refuses to start the
//! daemon with a clear error instead of running with nonsensical values.

use super::Config;
use crate::errors::{PowerError, Result};

pub fn validate(config: &Config) -> Result<()> {
    if config.battery.low_threshold_percent <= config.battery.critical_threshold_percent {
        return Err(PowerError::Config(format!(
            "battery.low_threshold_percent ({}) must be greater than battery.critical_threshold_percent ({})",
            config.battery.low_threshold_percent, config.battery.critical_threshold_percent
        )));
    }

    if config.thermal.warning_temp_c >= config.thermal.critical_temp_c {
        return Err(PowerError::Config(format!(
            "thermal.warning_temp_c ({}) must be less than thermal.critical_temp_c ({})",
            config.thermal.warning_temp_c, config.thermal.critical_temp_c
        )));
    }

    if config.display.off_timeout_secs > 0 && config.display.dim_timeout_secs > config.display.off_timeout_secs {
        return Err(PowerError::Config(format!(
            "display.dim_timeout_secs ({}) should not exceed display.off_timeout_secs ({})",
            config.display.dim_timeout_secs, config.display.off_timeout_secs
        )));
    }

    if config.display.default_brightness_percent > 100 {
        return Err(PowerError::Config("display.default_brightness_percent must be 0-100".into()));
    }

    let is_builtin = ["performance", "balanced", "powersave"].contains(&config.profiles.default_profile.as_str());
    let has_custom_match = config.profiles.profile.iter().any(|p| p.name == config.profiles.default_profile);
    if !is_builtin && !has_custom_match {
        return Err(PowerError::Config(format!(
            "profiles.default_profile '{}' is not a built-in profile and has no matching [[profile]] entry in profiles.toml",
            config.profiles.default_profile
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(validate(&Config::default()).is_ok());
    }

    #[test]
    fn low_below_critical_is_rejected() {
        let mut config = Config::default();
        config.battery.low_threshold_percent = 3;
        config.battery.critical_threshold_percent = 5;
        assert!(validate(&config).is_err());
    }

    #[test]
    fn unknown_default_profile_is_rejected() {
        let mut config = Config::default();
        config.profiles.default_profile = "does-not-exist".into();
        assert!(validate(&config).is_err());
    }
}
