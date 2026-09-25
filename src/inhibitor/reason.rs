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
    /// Give the holder a grace period to save state, then proceed anyway.
    /// v0 treats Delay identically to Block (see docs/ipc.md "Inhibitors"
    /// for the planned grace-period semantics); tracked separately so the
    /// wire format doesn't need to change when that lands.
    Delay,
}
