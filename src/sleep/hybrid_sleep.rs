//! Suspend to both: write RAM state to disk *and* suspend to RAM, so a
//! depleted battery during sleep still resumes from the disk image.

use super::sleep_state;
use crate::errors::{PowerError, Result};

pub async fn hybrid_sleep() -> Result<()> {
    let disk_modes = crate::hardware::sysfs::read_trimmed("/sys/power/disk").unwrap_or_default();
    if !disk_modes.contains("suspend") {
        return Err(PowerError::Unsupported("kernel does not report hybrid-sleep ('suspend' disk mode) support".into()));
    }
    tracing::info!("hybrid-sleeping");
    tokio::task::spawn_blocking(|| {
        sleep_state::write_disk_mode("suspend")?;
        sleep_state::write_state("disk")
    })
    .await
    .map_err(|e| PowerError::Internal(format!("hybrid-sleep task panicked: {e}")))??;
    tracing::info!("resumed from hybrid sleep");
    Ok(())
}
