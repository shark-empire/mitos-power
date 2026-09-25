//! Cross-zone aggregate math (zones.rs owns per-zone raw reads).

use super::zones::ThermalZoneInfo;

pub fn highest_c(zones: &[ThermalZoneInfo]) -> Option<f32> {
    zones.iter().filter_map(|z| z.temperature_c).fold(None, |acc, t| Some(acc.map_or(t, |a: f32| a.max(t))))
}

pub fn average_c(zones: &[ThermalZoneInfo]) -> Option<f32> {
    let values: Vec<f32> = zones.iter().filter_map(|z| z.temperature_c).collect();
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f32>() / values.len() as f32)
}
