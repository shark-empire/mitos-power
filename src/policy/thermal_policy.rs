//! Thermal Warning tier -> drop to powersave + notify. Critical is handled
//! immediately by `thermal::protection` at refresh time (bypassing the
//! policy-tick pipeline entirely), since an emergency shutdown shouldn't
//! wait for the next scheduled policy evaluation -- this policy only ever
//! sees Warning-level asks.

use crate::config::Config;
use crate::manager::policy::{Action, Policy};
use crate::manager::state::PowerStateSnapshot;
use crate::thermal::ThermalLevel;

pub struct ThermalPolicy;

impl Policy for ThermalPolicy {
    fn name(&self) -> &'static str {
        "thermal"
    }

    fn evaluate(&self, state: &PowerStateSnapshot, _config: &Config) -> Vec<Action> {
        match state.thermal_level {
            ThermalLevel::Warning => vec![
                Action::SetProfile("powersave".into()),
                Action::Notify { title: "Running hot".into(), body: "Switched to Power Saver to cool down".into() },
            ],
            ThermalLevel::Nominal | ThermalLevel::Critical => Vec::new(),
        }
    }
}
