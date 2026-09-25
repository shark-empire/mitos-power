//! Percentage-based brightness control, wrapping whichever backlight
//! device(s) this machine exposes. Uses the first discovered device as
//! primary; multi-display brightness fan-out is a documented future
//! extension (see docs/architecture.md).

use super::backlight::BacklightDevice;
use crate::errors::{PowerError, Result};

pub struct BrightnessController {
    device: Option<BacklightDevice>,
}

impl BrightnessController {
    pub fn new() -> Result<Self> {
        let devices = BacklightDevice::discover()?;
        Ok(Self { device: devices.into_iter().next() })
    }

    pub fn is_supported(&self) -> bool {
        self.device.is_some()
    }

    pub fn get_percent(&self) -> Result<u8> {
        let dev = self.device.as_ref().ok_or_else(|| PowerError::Unsupported("no backlight device".into()))?;
        let raw = dev.current_raw()?;
        Ok(((raw as f64 / dev.max_brightness.max(1) as f64) * 100.0).round() as u8)
    }

    pub fn set_percent(&self, percent: u8) -> Result<()> {
        let dev = self.device.as_ref().ok_or_else(|| PowerError::Unsupported("no backlight device".into()))?;
        let percent = percent.min(100);
        let raw = ((percent as f64 / 100.0) * dev.max_brightness as f64).round() as u32;
        dev.set_raw(raw)
    }
}
