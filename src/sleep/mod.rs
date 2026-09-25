//! Suspend / hibernate / hybrid-sleep / suspend-then-hibernate, implemented
//! against the kernel's raw `/sys/power/*` interface directly (mitos-power
//! does not shell out to systemd-logind -- MITOS has its own service
//! manager, mitos-services).

pub mod hibernate;
pub mod hybrid_sleep;
pub mod sleep_state;
pub mod suspend;
pub mod suspend_then_hibernate;
pub mod wake;
