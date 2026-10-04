use super::transition::TransitionBackend;
use crate::errors::{PowerError, Result};
use crate::logging::audit;

pub async fn reboot(requester: &str, force: bool, backend: TransitionBackend) -> Result<()> {
    audit::log_privileged_action(requester, "reboot", force);
    tracing::warn!("rebooting via {backend:?} backend (requested by {requester}, force={force})");

    tokio::task::spawn_blocking(move || backend.reboot())
        .await
        .map_err(|e| PowerError::Internal(format!("reboot task panicked: {e}")))??;

    Ok(())
}
