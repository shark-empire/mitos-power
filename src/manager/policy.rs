//! The policy engine: pure functions of (current state, config) -> actions.
//! This is what makes battery/thermal/profile-switching behavior
//! policy-based rather than hardcoded per the spec's design goal.
//!
//! Not every `policy/*` module goes through this generic engine -- lid and
//! idle actions are event-driven (evaluated once, right when the
//! triggering event happens) rather than re-evaluated every poll tick, so
//! `policy::lid_policy` and `policy::idle_policy` are called directly by
//! `devices::lid` / `idle::detector` instead. `policy::sleep_policy` is an
//! advisory helper consulted by `PowerManager::suspend()`, not an
//! independent action producer. See docs/architecture.md.

use crate::config::Config;
use crate::manager::state::PowerStateSnapshot;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Action {
    SetProfile(String),
    Suspend,
    Hibernate,
    Shutdown,
    Notify { title: String, body: String },
}

pub trait Policy: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, state: &PowerStateSnapshot, config: &Config) -> Vec<Action>;
}

pub struct PolicyEngine {
    policies: Vec<Box<dyn Policy>>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: vec![
                Box::new(crate::policy::battery_policy::BatteryPolicy),
                Box::new(crate::policy::thermal_policy::ThermalPolicy),
                Box::new(crate::policy::profile_policy::ProfilePolicy),
            ],
        }
    }

    pub fn evaluate(&self, state: &PowerStateSnapshot, config: &Config) -> Vec<Action> {
        self.policies.iter().flat_map(|p| p.evaluate(state, config)).collect()
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}
