//! Power profiles: the policy-based (not hardcoded) system for switching
//! CPU/display/background behavior as a unit. "performance", "balanced"
//! and "powersave" are built in; anything else comes from profiles.toml.

pub mod balanced;
pub mod custom;
pub mod manager;
pub mod performance;
pub mod powersave;
pub mod profile;

pub use manager::ProfileManager;
pub use profile::{ProfileInfo, ProfileKind, ProfileSettings};
