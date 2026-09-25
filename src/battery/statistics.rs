//! Rolling in-memory battery statistics used to smooth noisy instantaneous
//! readings. Long-term persisted history lives in `persistence::history`;
//! this is just the live window backing `GetPowerState`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

const WINDOW: usize = 30;

#[derive(Debug, Clone)]
struct Sample {
    at: Instant,
    percent: f32,
    power_w: Option<f64>,
}

#[derive(Debug, Default)]
pub struct BatteryStats {
    samples: VecDeque<Sample>,
}

impl BatteryStats {
    pub fn push(&mut self, percent: f32, power_w: Option<f64>) {
        self.samples.push_back(Sample { at: Instant::now(), percent, power_w });
        while self.samples.len() > WINDOW {
            self.samples.pop_front();
        }
    }

    pub fn average_power_w(&self) -> Option<f64> {
        let values: Vec<f64> = self.samples.iter().filter_map(|s| s.power_w).collect();
        if values.is_empty() {
            return None;
        }
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }

    pub fn percent_drop_rate_per_hour(&self) -> Option<f32> {
        let first = self.samples.front()?;
        let last = self.samples.back()?;
        let elapsed = last.at.duration_since(first.at);
        if elapsed < Duration::from_secs(30) {
            return None;
        }
        let delta = first.percent - last.percent;
        Some(delta / (elapsed.as_secs_f32() / 3600.0))
    }
}
