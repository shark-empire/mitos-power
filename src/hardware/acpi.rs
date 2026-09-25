//! ACPI-specific helpers.
//!
//! Most ACPI information on modern Linux is exposed through sysfs
//! (`/sys/class/power_supply`, `/sys/class/thermal`) rather than requiring
//! direct ACPI table parsing. mitos-power intentionally never talks to ACPI
//! tables itself; it relies on the kernel's ACPI driver having already done
//! that. This module only isolates the couple of ACPI-only quirks that
//! don't have a clean modern sysfs class.

use crate::errors::Result;
use crate::hardware::sysfs;
use std::path::Path;

const ACPI_BUTTON_LID: &str = "/proc/acpi/button/lid";

/// Legacy fallback lid-state read for kernels/configurations where the
/// input subsystem doesn't surface a lid switch but old-style
/// `/proc/acpi` does. Prefer `devices::lid` (evdev/uevent based); this is
/// a best-effort fallback only, used if that path finds nothing.
pub fn legacy_lid_state() -> Option<bool> {
    let base = Path::new(ACPI_BUTTON_LID);
    if !base.exists() {
        return None;
    }
    for entry in std::fs::read_dir(base).ok()?.flatten() {
        let state_file = entry.path().join("state");
        if let Ok(contents) = sysfs::read_trimmed(&state_file) {
            return Some(contents.to_lowercase().contains("closed"));
        }
    }
    None
}

pub fn is_acpi_available() -> bool {
    Path::new("/sys/firmware/acpi").exists()
}

pub fn acpi_sleep_states() -> Result<Vec<String>> {
    let raw = sysfs::read_trimmed("/sys/power/state")?;
    Ok(raw.split_whitespace().map(str::to_string).collect())
}
