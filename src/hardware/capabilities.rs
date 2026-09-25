//! Runtime detection of what this machine's kernel/hardware actually
//! supports, so the IPC layer and CLI can hide affordances (e.g.
//! hibernate) that would just fail on this box.

use crate::hardware::sysfs;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize)]
pub struct HardwareCapabilities {
    pub suspend: bool,
    pub hibernate: bool,
    pub hybrid_sleep: bool,
    pub backlight: bool,
    pub battery: bool,
    pub thermal_zones: bool,
    pub cpu_governor: bool,
    pub cpu_boost: bool,
    pub rtc_wakealarm: bool,
}

fn has_entries(dir: &str) -> bool {
    Path::new(dir).read_dir().map(|mut d| d.next().is_some()).unwrap_or(false)
}

impl HardwareCapabilities {
    pub fn detect() -> Self {
        let states = sysfs::read_trimmed("/sys/power/state").unwrap_or_default();
        let disk_modes = sysfs::read_trimmed("/sys/power/disk").unwrap_or_default();

        Self {
            suspend: states.contains("mem") || states.contains("freeze"),
            hibernate: states.contains("disk"),
            hybrid_sleep: disk_modes.contains("suspend"),
            backlight: has_entries("/sys/class/backlight"),
            battery: has_entries("/sys/class/power_supply"),
            thermal_zones: has_entries("/sys/class/thermal"),
            cpu_governor: Path::new("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor").exists(),
            cpu_boost: Path::new("/sys/devices/system/cpu/cpufreq/boost").exists(),
            rtc_wakealarm: Path::new("/sys/class/rtc/rtc0/wakealarm").exists(),
        }
    }
}
