//! Confirms `ChargingState` is reachable and behaves correctly through the
//! crate's public boundary (src/battery/charging.rs has the exhaustive
//! parsing unit tests; this is the "does the public re-export work" check).

use mitos_power::battery::ChargingState;

#[test]
fn charging_state_is_publicly_reachable_and_displays_lowercase() {
    let state: ChargingState = "Charging".into();
    assert_eq!(state, ChargingState::Charging);
    assert_eq!(state.to_string(), "charging");
}
