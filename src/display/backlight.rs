//! Raw backlight device access via /sys/class/backlight/<name>.

use crate::errors::{PowerError, Result};
use crate::hardware::sysfs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BacklightDevice {
    pub id: String,
    pub path: PathBuf,
    pub max_brightness: u32,
}

impl BacklightDevice {
    pub fn discover() -> Result<Vec<BacklightDevice>> {
        let mut out = Vec::new();
        for path in sysfs::list_all("/sys/class/backlight")? {
            let id = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let max_brightness = sysfs::read_u64(path.join("max_brightness"))? as u32;
            out.push(BacklightDevice { id, path, max_brightness });
        }
        Ok(out)
    }

    pub fn current_raw(&self) -> Result<u32> {
        Ok(sysfs::read_u64(self.path.join("brightness"))? as u32)
    }

    pub fn set_raw(&self, value: u32) -> Result<()> {
        if value > self.max_brightness {
            return Err(PowerError::InvalidParams(format!(
                "brightness {value} exceeds max_brightness {}",
                self.max_brightness
            )));
        }
        sysfs::write_string(self.path.join("brightness"), &value.to_string())
    }
}
