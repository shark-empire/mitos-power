//! Ends the current user session without powering the machine off.
//!
//! mitos-power does not own sessions -- that's mitos-session's job. This
//! is the documented integration seam: mitos-power should ask
//! mitos-session (over mitos-session's own IPC socket) to end the active
//! session. That client is not implemented in this crate yet; see
//! audit.md.

use crate::errors::Result;
use crate::logging::audit;

pub async fn logout(requester: &str) -> Result<()> {
    audit::log_privileged_action(requester, "logout", false);
    tracing::info!("logout requested by {requester}; mitos-session integration not yet implemented");
    Ok(())
}
