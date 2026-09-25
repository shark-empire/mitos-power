//! Settings that should survive a daemon restart (last active profile,
//! last brightness) -- separate from the compiled-in config.toml defaults.

use super::database::Database;
use crate::errors::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersistedSettings {
    pub last_profile: Option<String>,
    pub last_brightness_percent: Option<u8>,
}

const TABLE: &str = "settings";

pub fn load(db: &Database) -> Result<PersistedSettings> {
    db.load(TABLE)
}

pub fn save(db: &Database, settings: &PersistedSettings) -> Result<()> {
    db.save(TABLE, settings)
}
