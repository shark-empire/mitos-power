//! Durable-ish audit trail for privileged actions: who requested it, what
//! it was, and whether it was forced. Currently emitted through `tracing`
//! at a dedicated "audit" target rather than mitos-power managing its own
//! rotated log file -- point a log collector (journald, or MITOS's own
//! logging component once it exists) at that target to capture just these
//! lines. A dedicated on-disk audit log with rotation is a natural
//! follow-up; see audit.md.

pub fn log_privileged_action(requester: &str, action: &str, forced: bool) {
    tracing::info!(target: "audit", requester, action, forced, "privileged action");
}
