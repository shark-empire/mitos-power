//! `Shutdown` is the graceful, audited entry point; it currently delegates
//! straight to `poweroff` (MITOS doesn't yet distinguish "shutdown" from
//! "poweroff" the way old ACPI systems with soft-off vs. true power-cut
//! did -- both fully power the machine off).

use crate::errors::Result;
use crate::logging::audit;

pub async fn shutdown(requester: &str, force: bool) -> Result<()> {
    audit::log_privileged_action(requester, "shutdown", force);
    tracing::warn!("system shutdown requested by {requester} (force={force})");
    super::poweroff::poweroff(requester).await
}
