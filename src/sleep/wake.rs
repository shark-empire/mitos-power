//! Post-resume restoration, run after every suspend/hibernate/hybrid-sleep
//! /suspend-then-hibernate call regardless of which one it was.

use crate::errors::Result;
use crate::manager::PowerManager;

pub async fn on_resume(manager: &PowerManager) -> Result<()> {
    tracing::info!("post-resume: restoring power state");

    let profile = manager.get_profile().await;
    manager.cpu.apply_governor(&profile.settings.cpu_governor)?;
    manager.cpu.set_boost(profile.settings.cpu_boost)?;

    manager.report_activity().await;
    // Deliberately the policy-free refresh, not `refresh_and_run_policy`:
    // see that method's doc comment for why running the policy engine
    // synchronously on resume is unsafe (it could re-trigger the very
    // suspend/hibernate that just happened). The regular poll tick will
    // run the policy engine again a few seconds later regardless.
    manager.refresh_hardware_state().await;
    Ok(())
}
