//! Online/offline core accounting.

use crate::hardware::sysfs;

/// Parses the "/sys/devices/system/cpu/present" range (e.g. "0-7") into a
/// count. Falls back to 1 if unreadable/unparseable rather than panicking.
pub fn present_count() -> u32 {
    sysfs::read_trimmed("/sys/devices/system/cpu/present").ok().and_then(|s| parse_range_count(&s)).unwrap_or(1)
}

fn parse_range_count(s: &str) -> Option<u32> {
    let (lo, hi) = s.trim().split_once('-')?;
    let lo: u32 = lo.parse().ok()?;
    let hi: u32 = hi.parse().ok()?;
    hi.checked_sub(lo).map(|d| d + 1)
}

/// cpu0 has no "online" control file on most kernels (it can't be
/// offlined), so it always reports online.
pub fn is_online(cpu_index: u32) -> bool {
    if cpu_index == 0 {
        return true;
    }
    sysfs::read_trimmed_opt(format!("/sys/devices/system/cpu/cpu{cpu_index}/online"))
        .map(|v| v == "1")
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_range() {
        assert_eq!(parse_range_count("0-7"), Some(8));
        assert_eq!(parse_range_count("0-0"), Some(1));
        assert_eq!(parse_range_count("not a range"), None);
    }
}
