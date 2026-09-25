//! General wakeup-source enumeration via /sys/class/wakeup, plus the
//! kernel's system-wide wakeup event counter. Fully sysfs-based.

use crate::hardware::sysfs;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WakeupSource {
    pub name: String,
    pub active_count: Option<u64>,
    pub event_count: Option<u64>,
}

pub fn discover() -> Vec<WakeupSource> {
    let mut out = Vec::new();
    for path in sysfs::list_all("/sys/class/wakeup").unwrap_or_default() {
        let name = sysfs::read_trimmed_opt(path.join("name"))
            .unwrap_or_else(|| path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
        out.push(WakeupSource {
            name,
            active_count: sysfs::read_u64_opt(path.join("active_count")),
            event_count: sysfs::read_u64_opt(path.join("event_count")),
        });
    }
    out
}

/// The kernel's running total of wakeup events since boot -- a coarse
/// "something just woke the machine" signal.
pub fn wakeup_count() -> Option<u64> {
    sysfs::read_u64_opt("/sys/power/wakeup_count")
}
