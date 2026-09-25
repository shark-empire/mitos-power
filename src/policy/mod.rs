//! Policy modules. `battery_policy`, `thermal_policy` and `profile_policy`
//! implement `manager::policy::Policy` and run on every poll tick via
//! `PolicyEngine`. `lid_policy` and `idle_policy` are event-driven (called
//! directly by `devices::lid` / `idle::detector` right when the triggering
//! event happens) rather than re-evaluated on a timer. `sleep_policy` is an
//! advisory helper, not an independent action producer. See
//! docs/architecture.md "Policy engine".

pub mod battery_policy;
pub mod idle_policy;
pub mod lid_policy;
pub mod profile_policy;
pub mod sleep_policy;
pub mod thermal_policy;
