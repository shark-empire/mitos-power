//! Panic-free helpers for reading and writing Linux sysfs attribute files.

use crate::errors::{PowerError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Read a sysfs file and return its contents, trimmed of trailing whitespace.
pub fn read_trimmed(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .map_err(|e| PowerError::Hardware(format!("reading {}: {e}", path.display())))?;
    Ok(contents.trim().to_string())
}

pub fn read_u64(path: impl AsRef<Path>) -> Result<u64> {
    let raw = read_trimmed(path.as_ref())?;
    raw.parse::<u64>()
        .map_err(|e| PowerError::Hardware(format!("parsing {} as u64: {e}", path.as_ref().display())))
}

pub fn read_i64(path: impl AsRef<Path>) -> Result<i64> {
    let raw = read_trimmed(path.as_ref())?;
    raw.parse::<i64>()
        .map_err(|e| PowerError::Hardware(format!("parsing {} as i64: {e}", path.as_ref().display())))
}

/// Best-effort read: `None` instead of an error when the attribute is
/// missing, which is normal -- coverage varies a lot between vendors.
pub fn read_u64_opt(path: impl AsRef<Path>) -> Option<u64> {
    read_u64(path).ok()
}

pub fn read_trimmed_opt(path: impl AsRef<Path>) -> Option<String> {
    read_trimmed(path).ok()
}

pub fn write_string(path: impl AsRef<Path>, value: &str) -> Result<()> {
    let path = path.as_ref();
    fs::write(path, value)
        .map_err(|e| PowerError::Hardware(format!("writing '{value}' to {}: {e}", path.display())))
}

pub fn exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// List immediate children of `dir` whose name starts with `prefix`, e.g.
/// listing `/sys/class/power_supply` for entries starting with "BAT".
/// Returns an empty list (not an error) if `dir` itself doesn't exist.
pub fn list_matching(dir: impl AsRef<Path>, prefix: &str) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    let mut out = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(out),
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(prefix) {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}

/// List every immediate child directory of `dir`, regardless of name.
pub fn list_all(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    list_matching(dir, "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn read_trimmed_strips_newline() {
        let dir = std::env::temp_dir().join(format!("mitos-power-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("val");
        std::fs::File::create(&file).unwrap().write_all(b"42\n").unwrap();
        assert_eq!(read_trimmed(&file).unwrap(), "42");
        assert_eq!(read_u64(&file).unwrap(), 42);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_matching_missing_dir_is_empty_not_error() {
        let result = list_matching("/definitely/does/not/exist", "BAT").unwrap();
        assert!(result.is_empty());
    }
}
