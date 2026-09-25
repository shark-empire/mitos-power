//! A single battery as seen through /sys/class/power_supply/BATx.

use crate::errors::Result;
use crate::hardware::sysfs;
use std::path::PathBuf;

const POWER_SUPPLY_CLASS: &str = "/sys/class/power_supply";

/// Raw values last read from one battery power-supply device. Fields beyond
/// `id`/`path` are refreshed on every `refresh()`; derived/unit-converted
/// values live in the sibling modules (capacity, health, ...).
#[derive(Debug, Clone)]
pub struct BatteryDevice {
    pub id: String,
    pub path: PathBuf,
    pub present: bool,
    pub status_raw: String,
    pub capacity_percent: Option<u8>,
    pub energy_now_uwh: Option<u64>,
    pub energy_full_uwh: Option<u64>,
    pub energy_full_design_uwh: Option<u64>,
    pub charge_now_uah: Option<u64>,
    pub charge_full_uah: Option<u64>,
    pub charge_full_design_uah: Option<u64>,
    pub power_now_uw: Option<u64>,
    pub voltage_now_uv: Option<u64>,
    pub current_now_ua: Option<u64>,
    pub temp_decidegrees: Option<i64>,
    pub cycle_count: Option<u64>,
    pub technology: Option<String>,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
}

impl BatteryDevice {
    /// Discover every `BAT*` entry under the power_supply class.
    pub fn discover() -> Result<Vec<BatteryDevice>> {
        let mut out = Vec::new();
        for path in sysfs::list_matching(POWER_SUPPLY_CLASS, "BAT")? {
            let id = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let mut dev = BatteryDevice::empty(id, path);
            dev.refresh()?;
            out.push(dev);
        }
        Ok(out)
    }

    fn empty(id: String, path: PathBuf) -> Self {
        Self {
            id,
            path,
            present: false,
            status_raw: String::new(),
            capacity_percent: None,
            energy_now_uwh: None,
            energy_full_uwh: None,
            energy_full_design_uwh: None,
            charge_now_uah: None,
            charge_full_uah: None,
            charge_full_design_uah: None,
            power_now_uw: None,
            voltage_now_uv: None,
            current_now_ua: None,
            temp_decidegrees: None,
            cycle_count: None,
            technology: None,
            manufacturer: None,
            model_name: None,
        }
    }

    /// Re-read every attribute from sysfs. Individual attributes absent on
    /// this hardware simply stay `None` rather than failing the whole
    /// refresh -- coverage varies a lot between vendors and battery chemistries.
    pub fn refresh(&mut self) -> Result<()> {
        let p = |name: &str| self.path.join(name);

        self.present = sysfs::read_trimmed(p("present")).map(|s| s == "1").unwrap_or(true);
        self.status_raw = sysfs::read_trimmed(p("status")).unwrap_or_else(|_| "Unknown".into());
        self.capacity_percent = sysfs::read_u64_opt(p("capacity")).map(|v| v.min(100) as u8);

        // Energy-based (uWh) and charge-based (uAh) reporting are mutually
        // exclusive depending on the fuel gauge; read whichever exists.
        self.energy_now_uwh = sysfs::read_u64_opt(p("energy_now"));
        self.energy_full_uwh = sysfs::read_u64_opt(p("energy_full"));
        self.energy_full_design_uwh = sysfs::read_u64_opt(p("energy_full_design"));
        self.charge_now_uah = sysfs::read_u64_opt(p("charge_now"));
        self.charge_full_uah = sysfs::read_u64_opt(p("charge_full"));
        self.charge_full_design_uah = sysfs::read_u64_opt(p("charge_full_design"));

        self.power_now_uw = sysfs::read_u64_opt(p("power_now"));
        self.voltage_now_uv = sysfs::read_u64_opt(p("voltage_now"));
        self.current_now_ua = sysfs::read_u64_opt(p("current_now"));
        self.temp_decidegrees = sysfs::read_i64(p("temp")).ok();
        self.cycle_count = sysfs::read_u64_opt(p("cycle_count"));

        self.technology = sysfs::read_trimmed_opt(p("technology"));
        self.manufacturer = sysfs::read_trimmed_opt(p("manufacturer"));
        self.model_name = sysfs::read_trimmed_opt(p("model_name"));

        Ok(())
    }
}
