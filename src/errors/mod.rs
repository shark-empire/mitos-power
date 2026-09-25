//! Centralized error type for mitos-power. Every subsystem returns
//! `crate::errors::Result<T>` so the IPC layer has one place to map errors
//! onto wire error codes (see `ipc::server::error_code`).

mod error;

pub use error::{PowerError, Result};
