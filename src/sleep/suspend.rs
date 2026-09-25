//! Suspend to RAM (`/sys/power/state` = "mem", falling back to "freeze" if
//! "mem" isn't available -- e.g. some ARM boards only support s2idle).

use super::sleep_state;
use crate::errors::{PowerError, Result};

pub async fn suspend() -> Result<()> {
    let mode = preferred_mode()?;
    tracing::info!("suspending (mode={mode})");
    tokio::task::spawn_blocking(move || sleep_state::write_state(&mode))
        .await
        .map_err(|e| PowerError::Internal(format!("suspend task panicked: {e}")))??;
    tracing::info!("resumed from suspend");
    Ok(())
}

fn preferred_mode() -> Result<String> {
    let states = sleep_state::available_states()?;
    if states.iter().any(|s| s == "mem") {
        Ok("mem".to_string())
    } else if states.iter().any(|s| s == "freeze") {
        Ok("freeze".to_string())
    } else {
        Err(PowerError::Unsupported("kernel does not report any suspend-capable /sys/power/state value".into()))
    }
}
