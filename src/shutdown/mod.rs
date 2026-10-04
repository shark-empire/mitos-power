//! Shutdown / reboot / poweroff / logout, plus the emergency path used by
//! thermal protection. Graceful path: audit log -> (session wind-down via
//! mitos-session, handled by `PowerManager`) -> the configured
//! `transition::TransitionBackend` (direct kernel call, or a request to
//! PID 1 so mitos-services stops services in order first).

pub mod emergency;
pub mod logout;
pub mod poweroff;
pub mod reboot;
pub mod shutdown;
pub mod transition;
