//! Client for mitos-session's IPC socket -- a completely different wire
//! protocol from mitos-power's own (length-prefixed bincode, not NDJSON;
//! see `protocol.rs` for why the types are mirrored rather than shared).
//!
//! **Not part of the original spec's file tree.** Added to close the gap
//! audit.md flagged from the very first pass: `Logout` did nothing, and a
//! suspend/shutdown that didn't originate from mitos-session skipped
//! locking sessions and winding them down. See docs/architecture.md
//! "mitos-session integration" for the whole design and its limits.

pub mod client;
pub mod protocol;

pub use client::{lock_all_sessions, terminate_all_sessions, terminate_sessions_for_uid, SessionClient, SessionOpOutcome};
