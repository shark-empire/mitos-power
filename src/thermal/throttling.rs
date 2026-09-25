//! Reads kernel-reported trip points, when the driver exposes them.
//! Read-only awareness -- mitos-power never writes trip points; the
//! kernel's own thermal driver enforces them regardless.

use crate::hardware::sysfs;
use std::path::Path;

/// Returns (first "passive" trip temp, first "critical"/"hot" trip temp),
/// scanning trip_point_0.. until the driver stops exposing more. Coverage
/// is very hardware-dependent, hence both are `Option`.
pub fn read_trip_points(zone_path: &Path) -> (Option<f32>, Option<f32>) {
    let mut passive = None;
    let mut critical = None;
    for i in 0..16 {
        let type_path = zone_path.join(format!("trip_point_{i}_type"));
        if !sysfs::exists(&type_path) {
            break;
        }
        let kind = sysfs::read_trimmed_opt(&type_path).unwrap_or_default();
        let temp = sysfs::read_i64(zone_path.join(format!("trip_point_{i}_temp"))).ok().map(|v| v as f32 / 1000.0);
        match kind.as_str() {
            "passive" if passive.is_none() => passive = temp,
            "critical" | "hot" => critical = temp.or(critical),
            _ => {}
        }
    }
    (passive, critical)
}

/// Weak signal only: a non-zero throttle count means throttling has
/// happened *at some point* since boot, not necessarily right now. Not
/// exposed on all platforms.
pub fn has_ever_throttled() -> bool {
    sysfs::read_u64_opt("/sys/devices/system/cpu/cpu0/thermal_throttle/core_throttle_count")
        .map(|c| c > 0)
        .unwrap_or(false)
}
