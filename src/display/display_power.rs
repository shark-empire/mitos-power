//! Screen on/dim/off state tracking.
//!
//! mitos-power does not own the display server, so it cannot blank a
//! monitor itself the way it can drive backlight brightness through sysfs.
//! Instead, this module tracks the *intended* power state (driven by
//! `idle::timeout`) and the daemon emits it as part of `IdleStateChanged` /
//! a dedicated event; mitos-gui subscribes and performs the actual
//! DPMS/DRM output power change on its compositor.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayPowerState {
    On,
    Dimmed,
    Off,
}

pub struct DisplayPowerController {
    state: DisplayPowerState,
}

impl DisplayPowerController {
    pub fn new() -> Self {
        Self { state: DisplayPowerState::On }
    }

    pub fn state(&self) -> DisplayPowerState {
        self.state
    }

    /// Returns `true` if this is an actual transition worth telling
    /// mitos-gui about.
    pub fn set_state(&mut self, new_state: DisplayPowerState) -> bool {
        if self.state == new_state {
            return false;
        }
        self.state = new_state;
        true
    }
}

impl Default for DisplayPowerController {
    fn default() -> Self {
        Self::new()
    }
}
