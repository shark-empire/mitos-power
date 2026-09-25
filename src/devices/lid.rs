//! Lid-switch polling.
//!
//! This reads `/proc/acpi/button/lid` (via `hardware::acpi::legacy_lid_state`)
//! on every daemon poll tick rather than watching a live evdev `SW_LID`
//! switch event, so a lid transition is detected within one poll interval
//! (a few seconds by default) instead of instantly. A live evdev watcher
//! would remove that latency; see audit.md.

use crate::ac::AcState;
use crate::hardware::acpi;
use crate::manager::PowerManager;
use crate::policy::lid_policy::{self, LidAction};
use std::sync::Arc;

/// Called on every daemon poll tick.
pub async fn poll(manager: &Arc<PowerManager>) {
    let Some(closed) = acpi::legacy_lid_state() else {
        return; // no lid switch on this machine, or not exposed via this interface
    };

    let previous = manager.get_lid_state().await;
    if previous == Some(closed) {
        return;
    }
    manager.set_lid_state(closed).await;

    let on_ac = manager.get_ac_state().await == AcState::Online;
    let lid_config = manager.config.read().await.sleep.lid.clone();
    let action = lid_policy::action_for(closed, on_ac, &lid_config);
    apply(manager, action).await;
}

async fn apply(manager: &Arc<PowerManager>, action: LidAction) {
    match action {
        LidAction::Ignore | LidAction::Wake => {}
        LidAction::Lock => tracing::info!("lid policy: lock (delegated to mitos-session)"),
        LidAction::Suspend => {
            if let Err(e) = manager.suspend("lid-policy").await {
                tracing::debug!("lid-triggered suspend skipped: {e}");
            }
        }
        LidAction::Hibernate => {
            if let Err(e) = manager.hibernate("lid-policy").await {
                tracing::debug!("lid-triggered hibernate skipped: {e}");
            }
        }
        LidAction::Shutdown => {
            if let Err(e) = manager.shutdown("lid-policy", false).await {
                tracing::debug!("lid-triggered shutdown skipped: {e}");
            }
        }
    }
}
