//! Low-level read/write of the kernel's power-state control files.

use crate::errors::Result;
use crate::hardware::sysfs;

const POWER_STATE: &str = "/sys/power/state";
const POWER_DISK: &str = "/sys/power/disk";
const RTC_WAKEALARM: &str = "/sys/class/rtc/rtc0/wakealarm";

/// Write to /sys/power/state. NOTE: this call blocks the calling OS thread
/// until the machine actually resumes -- callers must invoke this from
/// `tokio::task::spawn_blocking`, never directly on an async task, or it
/// will stall the whole tokio runtime for as long as the system is asleep.
pub fn write_state(state: &str) -> Result<()> {
    sysfs::write_string(POWER_STATE, state)
}

pub fn write_disk_mode(mode: &str) -> Result<()> {
    sysfs::write_string(POWER_DISK, mode)
}

pub fn available_states() -> Result<Vec<String>> {
    Ok(sysfs::read_trimmed(POWER_STATE)?.split_whitespace().map(String::from).collect())
}

/// Set an RTC wake alarm at an absolute Unix timestamp (seconds), clearing
/// any existing alarm first as the kernel interface requires.
pub fn set_wakealarm(unix_secs: u64) -> Result<()> {
    sysfs::write_string(RTC_WAKEALARM, "0")?;
    sysfs::write_string(RTC_WAKEALARM, &unix_secs.to_string())
}

pub fn clear_wakealarm() -> Result<()> {
    sysfs::write_string(RTC_WAKEALARM, "0")
}

/// Best-effort check for whether any swap is currently active, read from
/// `/proc/swaps`. Not a guarantee hibernate will work (swap could still be
/// too small for RAM, or `resume=` could be missing from the kernel
/// command line) -- just enough to give a clear, specific warning for the
/// single most common "why doesn't hibernate work" cause before attempting
/// the write. Returns `true` (assume swap present, stay quiet) if
/// `/proc/swaps` itself can't be read, since an inconclusive check
/// shouldn't produce a false warning.
pub fn has_active_swap() -> bool {
    match std::fs::read_to_string("/proc/swaps") {
        Ok(contents) => contents.lines().count() > 1, // header line + one per active swap
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_active_swap_does_not_panic_regardless_of_environment() {
        // No assertion on the specific value -- this sandbox/CI may or may
        // not have swap. The property under test is "doesn't panic".
        let _ = has_active_swap();
    }
}
