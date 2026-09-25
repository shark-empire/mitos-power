//! Whether idle-triggered actions (dim/off/auto-suspend) are currently
//! inhibited, per the central inhibitor registry. Consulted by
//! `daemon::event_loop::idle_tick` before acting on a tier transition.

use crate::inhibitor::{InhibitWhat, InhibitorManager};

pub async fn is_idle_inhibited(inhibitors: &InhibitorManager) -> bool {
    !inhibitors.blockers(InhibitWhat::Idle).await.is_empty()
}
