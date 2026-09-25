//! USB per-device wakeup-source control via
//! /sys/bus/usb/devices/*/power/wakeup ("enabled" | "disabled"). Fully
//! sysfs-based -- no evdev/uevent dependency, unlike lid/power_button.

use crate::errors::Result;
use crate::hardware::sysfs;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct UsbWakeupDevice {
    pub id: String,
    #[serde(skip)]
    path: PathBuf,
    pub wakeup_enabled: Option<bool>,
}

pub fn discover() -> Result<Vec<UsbWakeupDevice>> {
    let mut out = Vec::new();
    for path in sysfs::list_all("/sys/bus/usb/devices")? {
        let wakeup_path = path.join("power/wakeup");
        if !sysfs::exists(&wakeup_path) {
            continue; // not every USB "device" entry is a real wakeup-capable node (e.g. interfaces)
        }
        let id = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let wakeup_enabled = sysfs::read_trimmed_opt(&wakeup_path).map(|v| v == "enabled");
        out.push(UsbWakeupDevice { id, path, wakeup_enabled });
    }
    Ok(out)
}

pub fn set_wakeup(device: &UsbWakeupDevice, enabled: bool) -> Result<()> {
    sysfs::write_string(device.path.join("power/wakeup"), if enabled { "enabled" } else { "disabled" })
}
