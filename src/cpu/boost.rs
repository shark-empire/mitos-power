//! Turbo/boost control. Intel exposes one global on/off file; AMD's
//! pstate driver exposes it per policy instead, so we try both.

use crate::errors::{PowerError, Result};
use crate::hardware::sysfs;

const INTEL_BOOST: &str = "/sys/devices/system/cpu/cpufreq/boost";

pub fn set(enabled: bool) -> Result<()> {
    let value = if enabled { "1" } else { "0" };

    if sysfs::exists(INTEL_BOOST) {
        return sysfs::write_string(INTEL_BOOST, value);
    }

    let mut wrote_any = false;
    for policy in sysfs::list_matching("/sys/devices/system/cpu/cpufreq", "policy").unwrap_or_default() {
        let path = policy.join("boost");
        if sysfs::exists(&path) {
            sysfs::write_string(&path, value)?;
            wrote_any = true;
        }
    }

    if wrote_any {
        Ok(())
    } else {
        Err(PowerError::Unsupported("no boost control found (checked Intel global + AMD per-policy paths)".into()))
    }
}

pub fn is_enabled() -> Option<bool> {
    sysfs::read_trimmed_opt(INTEL_BOOST).map(|v| v == "1")
}
