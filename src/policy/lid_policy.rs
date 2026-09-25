//! Pure lid-switch decision: (closed?, on_ac?, config) -> action. Called
//! directly by `devices::lid` right when the lid state is observed to
//! change, not through the generic `PolicyEngine`.

use crate::config::LidConfig;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LidAction {
    Ignore,
    Lock,
    Suspend,
    Hibernate,
    Shutdown,
    Wake,
}

impl FromStr for LidAction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "ignore" => LidAction::Ignore,
            "lock" => LidAction::Lock,
            "suspend" => LidAction::Suspend,
            "hibernate" => LidAction::Hibernate,
            "shutdown" => LidAction::Shutdown,
            "wake" => LidAction::Wake,
            _ => return Err(()),
        })
    }
}

pub fn action_for(closed: bool, on_ac: bool, config: &LidConfig) -> LidAction {
    if !closed {
        return config.open_action.parse().unwrap_or(LidAction::Wake);
    }
    let raw = if on_ac { &config.close_action_ac } else { &config.close_action_battery };
    raw.parse().unwrap_or(LidAction::Suspend)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> LidConfig {
        LidConfig {
            close_action_ac: "lock".into(),
            close_action_battery: "suspend".into(),
            open_action: "wake".into(),
            external_monitor_ignore_lid: true,
        }
    }

    #[test]
    fn closing_on_battery_suspends() {
        assert_eq!(action_for(true, false, &config()), LidAction::Suspend);
    }

    #[test]
    fn closing_on_ac_locks_instead() {
        assert_eq!(action_for(true, true, &config()), LidAction::Lock);
    }

    #[test]
    fn opening_wakes_regardless_of_power_source() {
        assert_eq!(action_for(false, true, &config()), LidAction::Wake);
        assert_eq!(action_for(false, false, &config()), LidAction::Wake);
    }
}
