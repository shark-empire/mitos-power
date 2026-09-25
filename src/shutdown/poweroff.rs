use crate::errors::{PowerError, Result};
use crate::logging::audit;

pub async fn poweroff(requester: &str) -> Result<()> {
    audit::log_privileged_action(requester, "poweroff", false);
    tracing::warn!("powering off (requested by {requester})");

    tokio::task::spawn_blocking(|| {
        nix::unistd::sync();
        nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_POWER_OFF)
    })
    .await
    .map_err(|e| PowerError::Internal(format!("poweroff task panicked: {e}")))?
    .map_err(|e| PowerError::Hardware(format!("poweroff syscall failed: {e}")))?;

    Ok(())
}
