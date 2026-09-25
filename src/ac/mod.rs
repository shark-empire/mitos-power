//! AC adapter detection and state tracking.

pub mod adapter;
pub mod detection;
pub mod state;

pub use adapter::AcAdapter;
pub use state::{AcManager, AcState};
