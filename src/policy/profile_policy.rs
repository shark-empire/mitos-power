//! Automatic profile switching on AC plug/unplug. Only acts when the
//! current profile is one of the two "automatic" built-ins so it never
//! overrides a profile the user picked deliberately (performance) or a
//! custom profile.

use crate::config::Config;
use crate::manager::policy::{Action, Policy};
use crate::manager::state::PowerStateSnapshot;
use crate::profiles::ProfileKind;

pub struct ProfilePolicy;

impl Policy for ProfilePolicy {
    fn name(&self) -> &'static str {
        "profile"
    }

    fn evaluate(&self, state: &PowerStateSnapshot, _config: &Config) -> Vec<Action> {
        match (&state.profile, state.on_ac) {
            (ProfileKind::PowerSaver, true) => vec![Action::SetProfile("balanced".into())],
            (ProfileKind::Balanced, false) => Vec::new(), // balanced is fine on battery too
            _ => Vec::new(),
        }
    }
}
