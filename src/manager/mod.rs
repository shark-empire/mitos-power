//! The central orchestrator. `manager::manager::PowerManager` owns every
//! subsystem and is the single object the IPC server and policy engine
//! call into.

pub mod manager;
pub mod policy;
pub mod scheduler;
pub mod state;

pub use manager::PowerManager;
pub use state::PowerStateSnapshot;
