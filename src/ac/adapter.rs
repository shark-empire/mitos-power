//! A single AC/mains power-supply device as seen through
//! /sys/class/power_supply/<name> where type == "Mains".

use crate::errors::Result;
use crate::hardware::sysfs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AcAdapter {
    pub id: String,
    pub path: PathBuf,
    pub online: bool,
}

impl AcAdapter {
    pub fn refresh(&mut self) -> Result<()> {
        self.online = sysfs::read_u64_opt(self.path.join("online")).map(|v| v != 0).unwrap_or(false);
        Ok(())
    }
}
