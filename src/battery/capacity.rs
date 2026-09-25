//! Capacity percentage derivation, preferring energy/charge math over the
//! kernel's own `capacity` file so the value stays consistent with
//! `energy_now_wh`/`energy_full_wh` shown elsewhere in the same response.

use super::device::BatteryDevice;

pub fn percentage(dev: &BatteryDevice) -> Option<f32> {
    if let (Some(now), Some(full)) = (dev.energy_now_uwh, dev.energy_full_uwh) {
        if full > 0 {
            return Some((now as f32 / full as f32) * 100.0);
        }
    }
    if let (Some(now), Some(full)) = (dev.charge_now_uah, dev.charge_full_uah) {
        if full > 0 {
            return Some((now as f32 / full as f32) * 100.0);
        }
    }
    dev.capacity_percent.map(|p| p as f32)
}

pub fn energy_now_wh(dev: &BatteryDevice) -> Option<f64> {
    dev.energy_now_uwh.map(|v| v as f64 / 1_000_000.0)
}

pub fn energy_full_wh(dev: &BatteryDevice) -> Option<f64> {
    dev.energy_full_uwh.map(|v| v as f64 / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a bare `BatteryDevice` for unit tests without touching real
    /// sysfs. All fields are `pub`, so this is just a struct literal.
    fn fake_device() -> BatteryDevice {
        BatteryDevice {
            id: "TEST0".into(),
            path: std::path::PathBuf::new(),
            present: true,
            status_raw: "Discharging".into(),
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

    #[test]
    fn percentage_from_energy() {
        let mut d = fake_device();
        d.energy_now_uwh = Some(25_000_000);
        d.energy_full_uwh = Some(50_000_000);
        assert_eq!(percentage(&d), Some(50.0));
    }

    #[test]
    fn percentage_falls_back_to_kernel_capacity() {
        let mut d = fake_device();
        d.capacity_percent = Some(63);
        assert_eq!(percentage(&d), Some(63.0));
    }
}
