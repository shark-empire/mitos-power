//! Battery threshold -> action mapping, driven entirely by battery.toml
//! (no hardcoded percentages).

use crate::config::Config;
use crate::manager::policy::{Action, Policy};
use crate::manager::state::PowerStateSnapshot;

pub struct BatteryPolicy;

impl Policy for BatteryPolicy {
    fn name(&self) -> &'static str {
        "battery"
    }

    fn evaluate(&self, state: &PowerStateSnapshot, config: &Config) -> Vec<Action> {
        if state.on_ac {
            return Vec::new(); // thresholds only make sense while discharging
        }
        let Some(pct) = state.overall_percentage else {
            return Vec::new();
        };
        let battery = &config.battery;

        if pct <= battery.critical_threshold_percent as f32 {
            return match battery.critical_action.as_str() {
                "hibernate" => vec![Action::Hibernate],
                "suspend" => vec![Action::Suspend],
                "shutdown" => vec![Action::Shutdown],
                _ => vec![Action::Notify {
                    title: "Battery critical".into(),
                    body: format!("{pct:.0}% remaining"),
                }],
            };
        }

        if pct <= battery.low_threshold_percent as f32 && battery.low_action == "notify" {
            return vec![Action::Notify { title: "Battery low".into(), body: format!("{pct:.0}% remaining") }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::profiles::ProfileKind;
    use crate::thermal::ThermalLevel;

    fn state_with_percent(on_ac: bool, pct: f32) -> PowerStateSnapshot {
        PowerStateSnapshot {
            on_ac,
            batteries: vec![],
            overall_percentage: Some(pct),
            profile: ProfileKind::Balanced,
            lid_closed: None,
            display_brightness_percent: None,
            thermal_level: ThermalLevel::Nominal,
            active_inhibitors: 0,
            idle_seconds: 0,
        }
    }

    #[test]
    fn no_action_on_ac_even_if_percent_is_low() {
        let config = Config::default();
        let state = state_with_percent(true, 1.0);
        assert!(BatteryPolicy.evaluate(&state, &config).is_empty());
    }

    #[test]
    fn critical_on_battery_triggers_configured_action() {
        let mut config = Config::default();
        config.battery.critical_threshold_percent = 5;
        config.battery.critical_action = "hibernate".into();
        let state = state_with_percent(false, 3.0);
        assert_eq!(BatteryPolicy.evaluate(&state, &config), vec![Action::Hibernate]);
    }
}
