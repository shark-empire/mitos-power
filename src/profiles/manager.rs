//! Applies/selects profiles. Deliberately does *not* poke CPU/display
//! subsystems directly -- `PowerManager::set_profile` does that after
//! calling `set()` here, keeping "which profile is active" separate from
//! "what a profile switch does to the hardware".

use super::custom::CustomProfileToml;
use super::profile::{ProfileInfo, ProfileKind};
use super::{balanced, performance, powersave};
use crate::errors::{PowerError, Result};

pub struct ProfileManager {
    available: Vec<ProfileInfo>,
    current_name: String,
}

impl ProfileManager {
    pub fn new(default_profile: &str, custom: Vec<CustomProfileToml>) -> Result<Self> {
        let mut available = vec![
            ProfileInfo {
                name: "performance".into(),
                kind: ProfileKind::Performance,
                settings: performance::settings(),
                is_custom: false,
            },
            ProfileInfo {
                name: "balanced".into(),
                kind: ProfileKind::Balanced,
                settings: balanced::settings(),
                is_custom: false,
            },
            ProfileInfo {
                name: "powersave".into(),
                kind: ProfileKind::PowerSaver,
                settings: powersave::settings(),
                is_custom: false,
            },
        ];
        for c in custom {
            available.push(c.into());
        }

        if !available.iter().any(|p| p.name == default_profile) {
            return Err(PowerError::Config(format!(
                "default_profile '{default_profile}' is not defined in profiles.toml"
            )));
        }

        Ok(Self { available, current_name: default_profile.to_string() })
    }

    pub fn current(&self) -> &ProfileInfo {
        self.available
            .iter()
            .find(|p| p.name == self.current_name)
            .expect("current_name always refers to a profile in `available`")
    }

    pub fn list(&self) -> &[ProfileInfo] {
        &self.available
    }

    pub fn set(&mut self, name: &str) -> Result<&ProfileInfo> {
        if !self.available.iter().any(|p| p.name == name) {
            return Err(PowerError::NotFound(format!("profile '{name}'")));
        }
        self.current_name = name.to_string();
        Ok(self.current())
    }
}
