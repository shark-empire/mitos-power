//! Suspend to disk (`/sys/power/state` = "disk"). Requires swap sized for
//! RAM and resume= configured on the kernel command line; mitos-power
//! doesn't verify that here (see docs/troubleshooting.md).

use super::sleep_state;
use crate::errors::{PowerError, Result};

pub async fn hibernate() -> Result<()> {
    let states = sleep_state::available_states()?;
    if !states.iter().any(|s| s == "disk") {
        return Err(PowerError::Unsupported("kernel does not report hibernate ('disk') support".into()));
    }
    tracing::info!("hibernating");
    tokio::task::spawn_blocking(|| sleep_state::write_state("disk"))
        .await
        .map_err(|e| PowerError::Internal(format!("hibernate task panicked: {e}")))??;
    tracing::info!("resumed from hibernate");
    Ok(())
}
