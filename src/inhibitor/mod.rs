//! Power inhibitors: lets applications and system components temporarily
//! block suspend/shutdown/idle actions ("Movie is playing", "Backup
//! running", ...). See docs/ipc.md "Inhibitors".

pub mod inhibitor;
pub mod manager;
pub mod reason;

pub use inhibitor::{Inhibitor, InhibitorInfo};
pub use manager::InhibitorManager;
pub use reason::{InhibitMode, InhibitWhat};
