//! Per-CPU frequency introspection (read-only; governor.rs owns writes).

use crate::hardware::sysfs;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct CpuFrequency {
    pub current_khz: Option<u64>,
    pub min_khz: Option<u64>,
    pub max_khz: Option<u64>,
}

pub fn read(cpu_index: u32) -> CpuFrequency {
    let base = Path::new("/sys/devices/system/cpu").join(format!("cpu{cpu_index}/cpufreq"));
    CpuFrequency {
        current_khz: sysfs::read_u64_opt(base.join("scaling_cur_freq")),
        min_khz: sysfs::read_u64_opt(base.join("cpuinfo_min_freq")),
        max_khz: sysfs::read_u64_opt(base.join("cpuinfo_max_freq")),
    }
}
