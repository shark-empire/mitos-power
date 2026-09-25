//! scaling_governor read/write across every online CPU.

use crate::errors::Result;
use crate::hardware::sysfs;
use std::path::{Path, PathBuf};

fn is_numbered_cpu_dir(p: &Path) -> bool {
    let name = match p.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };
    match name.strip_prefix("cpu") {
        Some(rest) if !rest.is_empty() => rest.chars().all(|c| c.is_ascii_digit()),
        _ => false,
    }
}

fn cpu_paths() -> Vec<PathBuf> {
    sysfs::list_matching("/sys/devices/system/cpu", "cpu")
        .unwrap_or_default()
        .into_iter()
        .filter(|p| is_numbered_cpu_dir(p))
        .collect()
}

/// Sets the governor on every CPU that exposes the control file. Missing
/// per-CPU cpufreq directories (e.g. an offlined core) are skipped, not
/// treated as an error; only returns `Err` if *no* CPU accepted the write.
pub fn set_all(governor: &str) -> Result<()> {
    let mut last_err = None;
    let mut wrote_any = false;
    for cpu in cpu_paths() {
        let path = cpu.join("cpufreq/scaling_governor");
        if sysfs::exists(&path) {
            match sysfs::write_string(&path, governor) {
                Ok(()) => wrote_any = true,
                Err(e) => last_err = Some(e),
            }
        }
    }
    if wrote_any {
        Ok(())
    } else {
        Err(last_err.unwrap_or_else(|| crate::errors::PowerError::Unsupported("no cpufreq governor control found".into())))
    }
}

pub fn current() -> Option<String> {
    sysfs::read_trimmed_opt("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
}

pub fn available() -> Vec<String> {
    sysfs::read_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/scaling_available_governors")
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default()
}
