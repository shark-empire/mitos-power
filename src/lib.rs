//! mitos-power library crate.
//!
//! Both `src/main.rs` (the daemon) and `bin/mitos-powerctl.rs` (the CLI)
//! depend on this crate rather than duplicating types, so that the IPC
//! protocol client and server are always built from the exact same Rust
//! definitions. This is one deliberate deviation from the spec's file tree,
//! which only listed `src/main.rs` -- see README.md "Notes on structure".

pub mod ac;
pub mod battery;
pub mod config;
pub mod cpu;
pub mod daemon;
pub mod devices;
pub mod display;
pub mod errors;
pub mod hardware;
pub mod idle;
pub mod inhibitor;
pub mod ipc;
pub mod logging;
pub mod manager;
pub mod monitoring;
pub mod persistence;
pub mod policy;
pub mod profiles;
pub mod shutdown;
pub mod sleep;
pub mod thermal;
