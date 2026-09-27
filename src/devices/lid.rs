//! Lid-switch handling, two ways:
//!
//! - `poll()` reads `/proc/acpi/button/lid` on every daemon poll tick --
//!   works everywhere this fallback interface exists, with up to one poll
//!   interval (a few seconds by default) of latency.
//! - `handle_live_signal()` reacts instantly to a `SW_LID` evdev event from
//!   `hardware::evdev::spawn_watcher` when that's available.
//!
//! `daemon::event_loop` uses the live path when the evdev watcher started
//! successfully, falling back to `poll()` otherwise -- both funnel through
//! `apply_if_changed` so the actual lid *policy* is only implemented once.

use crate::ac::AcState;
use crate::hardware::acpi;
use crate::manager::PowerManager;
use crate::policy::lid_policy::{self, LidAction};
use std::sync::Arc;

/// Called on every daemon poll tick. A no-op once a live evdev watcher is
/// active for `SW_LID` specifically, since `handle_live_signal` will have
/// already applied any change -- `apply_if_changed`'s "did this actually
/// change" check makes calling both paths harmless rather than something
/// `event_loop` needs to coordinate.
pub async fn poll(manager: &Arc<PowerManager>) {
    let Some(closed) = acpi::legacy_lid_state() else {
        return; // no lid switch on this machine, or not exposed via this interface
    };
    apply_if_changed(manager, closed).await;
}

/// Called by `daemon::event_loop` when the live evdev watcher reports a
/// `SW_LID` transition directly.
pub async fn handle_live_signal(manager: &Arc<PowerManager>, closed: bool) {
    apply_if_changed(manager, closed).await;
}

async fn apply_if_changed(manager: &Arc<PowerManager>, closed: bool) {
    let previous = manager.get_lid_state().await;
    if previous == Some(closed) {
        return;
    }
    manager.set_lid_state(closed).await;

    let on_ac = manager.get_ac_state().await == AcState::Online;
    let lid_config = manager.config.read().await.sleep.lid.clone();
    let action = lid_policy::action_for(closed, on_ac, &lid_config);
    apply_action(manager, action).await;
}

async fn apply_action(manager: &Arc<PowerManager>, action: LidAction) {
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
