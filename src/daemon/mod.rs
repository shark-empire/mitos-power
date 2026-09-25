//! Daemon wiring: construct `PowerManager`, start the IPC server and the
//! hardware event loop, and handle signals.

pub mod daemon;
pub mod event_loop;
pub mod lifecycle;

pub use daemon::Daemon;
