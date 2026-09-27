//! Suspend to disk (`/sys/power/state` = "disk"). Requires swap sized for
//! RAM and resume= configured on the kernel command line -- `has_active_swap`
//! gives an early, specific warning for the most common failure cause, but
//! doesn't hard-block the attempt (a marginal-but-working setup shouldn't
//! be refused based on this coarse a check).

use super::sleep_state;
use crate::errors::{PowerError, Result};

pub async fn hibernate() -> Result<()> {
    let states = sleep_state::available_states()?;
    if !states.iter().any(|s| s == "disk") {
        return Err(PowerError::Unsupported("kernel does not report hibernate ('disk') support".into()));
    }
    if !sleep_state::has_active_swap() {
        tracing::warn!(
            "no active swap found in /proc/swaps -- hibernate will likely fail even though \
             the kernel lists 'disk' as supported; see docs/troubleshooting.md"
        );
    }
    tracing::info!("hibernating");
    tokio::task::spawn_blocking(|| sleep_state::write_state("disk"))
        .await
        .map_err(|e| PowerError::Internal(format!("hibernate task panicked: {e}")))??;
    tracing::info!("resumed from hibernate");
    Ok(())
}
