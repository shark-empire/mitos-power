//! Shutdown / reboot / poweroff / logout, plus the emergency path used by
//! thermal protection. Graceful paths: log/audit -> (future: notify
//! mitos-session/mitos-services) -> sync filesystems -> kernel syscall.

pub mod emergency;
pub mod logout;
pub mod poweroff;
pub mod reboot;
pub mod shutdown;
