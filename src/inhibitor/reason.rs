//! What an inhibitor blocks, and how strongly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InhibitWhat {
    Suspend,
    Shutdown,
    Idle,
    All,
}

impl InhibitWhat {
    /// Does an inhibitor requesting `self` block an action of kind `target`?
    pub fn covers(&self, target: InhibitWhat) -> bool {
        *self == InhibitWhat::All || *self == target
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InhibitMode {
    /// Refuse the action outright while the inhibitor is held.
    Block,
    /// Give the holder a grace period (`general.inhibitor_delay_grace_secs`)
    /// to release it, then proceed anyway regardless -- see
    /// `InhibitorManager::wait_for_delay_clear` and
    /// `PowerManager::ensure_not_inhibited`. Idle-triggered actions treat
    /// this the same as `Block` (see `idle::inhibitors`); Suspend/Shutdown
    /// give it the real grace-period behavior.
    Delay,
}
