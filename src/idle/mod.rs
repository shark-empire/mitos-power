//! Idle detection and the dim/off/suspend tiers it drives.

pub mod detector;
pub mod inhibitors;
pub mod policy;
pub mod timeout;

pub use detector::{IdleDetector, IdleState};
