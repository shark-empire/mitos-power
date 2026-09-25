//! Discovers AC/mains adapters. Naming varies a lot by vendor (AC, ACAD,
//! ADP0, ADP1, ...), so instead of matching on name prefix we check the
//! `type` attribute, which the kernel standardizes to "Mains" for AC
//! adapters (as opposed to "Battery" or "USB").

use super::adapter::AcAdapter;
use crate::errors::Result;
use crate::hardware::sysfs;

const POWER_SUPPLY_CLASS: &str = "/sys/class/power_supply";

pub fn discover() -> Result<Vec<AcAdapter>> {
    let mut out = Vec::new();
    for path in sysfs::list_all(POWER_SUPPLY_CLASS)? {
        let kind = sysfs::read_trimmed(path.join("type")).unwrap_or_default();
        if kind == "Mains" {
            let id = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let mut adapter = AcAdapter { id, path, online: false };
            adapter.refresh()?;
            out.push(adapter);
        }
    }
    Ok(out)
}
