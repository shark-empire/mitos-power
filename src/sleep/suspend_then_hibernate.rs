//! Suspend to RAM first; if nothing wakes the machine within
//! `delay_mins`, an RTC alarm fires and we follow through to hibernate
//! (saving battery on a machine left asleep for a long time). If the
//! machine wakes early (lid opened, power button, ...), it just resumes
//! normally and the pending alarm is cleared.
//!
//! This mirrors systemd-logind's suspend-then-hibernate semantics without
//! depending on systemd.

use super::sleep_state;
use crate::errors::{PowerError, Result};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub async fn suspend_then_hibernate(delay_mins: u64) -> Result<()> {
    let delay = Duration::from_secs(delay_mins.saturating_mul(60).max(60));
    let wake_at_unix = SystemTime::now()
        .checked_add(delay)
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .ok_or_else(|| PowerError::Internal("could not compute RTC wake time".into()))?
        .as_secs();

    tracing::info!("suspend-then-hibernate: suspending now, RTC alarm armed for {delay_mins}m from now");
    let started = Instant::now();

    tokio::task::spawn_blocking(move || -> Result<()> {
        sleep_state::set_wakealarm(wake_at_unix)?;
        sleep_state::write_state("mem")?; // blocks until resume
        sleep_state::clear_wakealarm()?;
        Ok(())
    })
    .await
    .map_err(|e| PowerError::Internal(format!("suspend-then-hibernate task panicked: {e}")))??;

    let slept_for = started.elapsed();
    if slept_for >= delay {
        tracing::info!("suspend-then-hibernate: RTC alarm elapsed ({slept_for:?}), following through to hibernate");
        super::hibernate::hibernate().await?;
    } else {
        tracing::info!("suspend-then-hibernate: woke early after {slept_for:?}, resuming normally");
    }
    Ok(())
}
