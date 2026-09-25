use crate::errors::{PowerError, Result};
use crate::logging::audit;

pub async fn reboot(requester: &str, force: bool) -> Result<()> {
    audit::log_privileged_action(requester, "reboot", force);
    tracing::warn!("rebooting (requested by {requester}, force={force})");

    tokio::task::spawn_blocking(|| {
        nix::unistd::sync();
        nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_AUTOBOOT)
    })
    .await
    .map_err(|e| PowerError::Internal(format!("reboot task panicked: {e}")))?
    .map_err(|e| PowerError::Hardware(format!("reboot syscall failed: {e}")))?;

    Ok(())
}
