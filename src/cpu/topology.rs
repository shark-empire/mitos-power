//! Basic core/package topology.

use crate::hardware::sysfs;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CoreTopology {
    pub core_id: Option<u32>,
    pub physical_package_id: Option<u32>,
}

pub fn read(cpu_index: u32) -> CoreTopology {
    let base = format!("/sys/devices/system/cpu/cpu{cpu_index}/topology");
    CoreTopology {
        core_id: sysfs::read_u64_opt(format!("{base}/core_id")).map(|v| v as u32),
        physical_package_id: sysfs::read_u64_opt(format!("{base}/physical_package_id")).map(|v| v as u32),
    }
}
