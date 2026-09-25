//! The IPC surface every other mitos-* component (and applications) talks
//! to. Full protocol reference: docs/ipc.md.

pub mod client;
pub mod events;
pub mod messages;
pub mod permissions;
pub mod protocol;
pub mod server;

pub use client::IpcClient;
pub use protocol::{ClientMessage, ErrorPayload, ServerMessage};
pub use server::IpcServer;
