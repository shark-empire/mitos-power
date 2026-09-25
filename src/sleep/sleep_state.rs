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
