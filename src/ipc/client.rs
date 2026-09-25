//! Async IPC client. Used by `mitos-powerctl`, and importable as a library
//! by any other Rust mitos-* component that wants to talk to mitos-power
//! (e.g. mitos-session, mitos-settings) instead of hand-rolling the socket
//! protocol.

use super::protocol::{ClientMessage, ErrorPayload, ServerMessage};
use crate::errors::{PowerError, Result};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use uuid::Uuid;

pub struct IpcClient {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

#[derive(Debug, Clone)]
pub struct EventEnvelope {
    pub event: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl IpcClient {
    pub async fn connect(socket_path: &Path) -> Result<Self> {
        let stream = UnixStream::connect(socket_path)
            .await
            .map_err(|e| PowerError::Hardware(format!("connecting to {}: {e}", socket_path.display())))?;
        let (r, w) = stream.into_split();
        Ok(Self { reader: BufReader::new(r), writer: w })
    }

    async fn send(&mut self, msg: &ClientMessage) -> Result<()> {
        let mut line = serde_json::to_string(msg)?;
        line.push('\n');
        self.writer.write_all(line.as_bytes()).await?;
        Ok(())
    }

    /// Send a request and wait for its correlated response. Any `Event`
    /// messages received while waiting are silently skipped -- callers that
    /// need events should drive a separate connection via `subscribe()` +
    /// `next_event()`, since a single socket can be doing both but this
    /// helper is meant for simple request/response call sites like the CLI.
    pub async fn call(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = Uuid::new_v4().to_string();
        self.send(&ClientMessage::Request { id: id.clone(), method: method.to_string(), params }).await?;

        loop {
            let mut buf = String::new();
            let n = self.reader.read_line(&mut buf).await?;
            if n == 0 {
                return Err(PowerError::Hardware("connection closed by mitos-power".into()));
            }
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<ServerMessage>(trimmed)? {
                ServerMessage::Response { id: rid, ok, result, error } if rid == id => {
                    return if ok {
                        Ok(result.unwrap_or(serde_json::Value::Null))
                    } else {
                        let ErrorPayload { code, message } = error.unwrap_or(ErrorPayload {
                            code: "INTERNAL_ERROR".into(),
                            message: "server returned ok=false with no error payload".into(),
                        });
                        Err(PowerError::Internal(format!("{code}: {message}")))
                    };
                }
                // Response to a stale/foreign request id, or an Event while
                // we're waiting on a reply -- keep reading.
                _ => continue,
            }
        }
    }

    pub async fn subscribe(&mut self, events: Vec<String>) -> Result<()> {
        self.send(&ClientMessage::Subscribe { events }).await
    }

    pub async fn unsubscribe(&mut self, events: Vec<String>) -> Result<()> {
        self.send(&ClientMessage::Unsubscribe { events }).await
    }

    /// Blocks until the next `Event` arrives (or the connection closes).
    /// Only useful after `subscribe()`; call in a loop for a live event feed.
    pub async fn next_event(&mut self) -> Result<Option<EventEnvelope>> {
        loop {
            let mut buf = String::new();
            let n = self.reader.read_line(&mut buf).await?;
            if n == 0 {
                return Ok(None);
            }
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<ServerMessage>(trimmed)? {
                ServerMessage::Event { event, data, timestamp } => {
                    return Ok(Some(EventEnvelope { event, data, timestamp }))
                }
                ServerMessage::Response { .. } => continue, // stray reply on an event-only connection
            }
        }
    }
}
