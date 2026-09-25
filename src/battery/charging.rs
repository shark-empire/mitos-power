//! Charging-state parsing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargingState {
    Charging,
    Discharging,
    NotCharging,
    Full,
    Unknown,
}

impl From<&str> for ChargingState {
    fn from(raw: &str) -> Self {
        match raw.trim() {
            "Charging" => ChargingState::Charging,
            "Discharging" => ChargingState::Discharging,
            "Not charging" => ChargingState::NotCharging,
            "Full" => ChargingState::Full,
            _ => ChargingState::Unknown,
        }
    }
}

impl std::fmt::Display for ChargingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ChargingState::Charging => "charging",
            ChargingState::Discharging => "discharging",
            ChargingState::NotCharging => "not charging",
            ChargingState::Full => "full",
            ChargingState::Unknown => "unknown",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kernel_strings() {
        assert_eq!(ChargingState::from("Charging"), ChargingState::Charging);
        assert_eq!(ChargingState::from("Not charging"), ChargingState::NotCharging);
        assert_eq!(ChargingState::from("something weird"), ChargingState::Unknown);
    }
}
