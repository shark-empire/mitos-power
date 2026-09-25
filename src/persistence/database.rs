//! One JSON file per "table" under the state directory, with atomic
//! (write-tmp-then-rename) saves so a crash mid-write can't corrupt state.

use crate::errors::{PowerError, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;

pub struct Database {
    dir: PathBuf,
}

impl Database {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn path_for(&self, table: &str) -> PathBuf {
        self.dir.join(format!("{table}.json"))
    }

    /// Loads a table, returning `T::default()` if the file doesn't exist
    /// yet (first run) rather than erroring.
    pub fn load<T: DeserializeOwned + Default>(&self, table: &str) -> Result<T> {
        let path = self.path_for(table);
        if !path.exists() {
            return Ok(T::default());
        }
        let raw = std::fs::read_to_string(&path)?;
        serde_json::from_str(&raw)
            .map_err(|e| PowerError::Internal(format!("corrupt state file {}: {e}", path.display())))
    }

    pub fn save<T: Serialize>(&self, table: &str, value: &T) -> Result<()> {
        let path = self.path_for(table);
        let tmp_path = path.with_extension("json.tmp");
        let raw = serde_json::to_string_pretty(value)?;
        std::fs::write(&tmp_path, raw)?;
        std::fs::rename(&tmp_path, &path)?; // atomic on the same filesystem
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Sample {
        value: u32,
    }

    #[test]
    fn round_trips_and_defaults_when_missing() {
        let dir = std::env::temp_dir().join(format!("mitos-power-db-test-{}", std::process::id()));
        let db = Database::open(&dir).unwrap();

        let loaded: Sample = db.load("sample").unwrap();
        assert_eq!(loaded, Sample::default());

        db.save("sample", &Sample { value: 42 }).unwrap();
        let loaded: Sample = db.load("sample").unwrap();
        assert_eq!(loaded, Sample { value: 42 });

        let _ = std::fs::remove_dir_all(&dir);
    }
}
