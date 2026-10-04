use super::transition::TransitionBackend;
use crate::errors::{PowerError, Result};
use crate::logging::audit;

pub async fn poweroff(requester: &str, backend: TransitionBackend) -> Result<()> {
    audit::log_privileged_action(requester, "poweroff", false);
    tracing::warn!("powering off via {backend:?} backend (requested by {requester})");

    tokio::task::spawn_blocking(move || backend.poweroff())
        .await
        .map_err(|e| PowerError::Internal(format!("poweroff task panicked: {e}")))??;

    Ok(())
}
