//! A precise mirror of mitos-session's real wire protocol
//! (`Request`/`Response`/`Event`/`Message` and everything they
//! reference), reconstructed by reading mitos-session's actual source
//! (docs/ipc.md, src/ipc/{messages,protocol}.rs, plus the types those
//! reference) rather than by sharing a crate with it -- mitos-power and
//! mitos-session are two independent Cargo projects, which is the
//! normal situation for sibling system daemons integrating over a wire
//! protocol instead of a shared library.
//!
//! **CRITICAL: bincode has no field names or variant names on the wire.**
//! Every enum variant is encoded as a `u32` ordinal by *declaration
//! order*, and every struct's fields are encoded positionally. Every
//! type below must match mitos-session's real declaration order EXACTLY
//! -- field-for-field, variant-for-variant -- or messages will silently
//! decode as the *wrong value* rather than failing cleanly the way a
//! JSON shape mismatch would. (mitos-session's own source says the same
//! about its `AuthOutcome::Cancelled`: "MUST stay last... bincode
//! encodes enum variants by ordinal position".)
//!
//! Mirrored against mitos-session as uploaded on 2026-09-27. If that
//! project's wire types change, this file drifts out of sync *silently*
//! -- there is no compile-time link between the two crates. See
//! audit.md "mitos-session integration" for what that means in practice
//! and how to catch it (short version: an integration test against a
//! real running mitos-session, which this sandbox couldn't do).
//!
//! Only three request types are ever *sent* by mitos-power
//! (`ListSessions`, `LockSession`, `TerminateSession`), but the full
//! `Request`/`Response`/`Event` enums are mirrored anyway. That's
//! deliberate: variant ordinals depend on every earlier variant being
//! present, and mirroring the whole thing means this file can be checked
//! against mitos-session's `messages.rs` line-by-line rather than
//! requiring anyone to reason about which subset is "safe" to omit.

use crate::errors::{PowerError, Result};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub type SessionId = u32;
pub type ElevationRequestId = u64;

/// Same cap as mitos-session's own `protocol::MAX_MESSAGE_LEN`: a
/// hostile or broken peer can't make us allocate an arbitrarily large
/// buffer just by lying in the 4-byte length prefix.
pub const MAX_MESSAGE_LEN: u32 = 16 * 1024 * 1024;

// ---- leaf types (declaration order == wire order; do not reorder) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    Wayland,
    X11,
    Tty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthOutcome {
    Success,
    Failure { attempts_remaining: u32 },
    LockedOut { retry_after_secs: u64 },
    Error(String),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockReason {
    Manual,
    Idle,
    Suspend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InhibitWhat {
    Idle,
    Lock,
    Suspend,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InhibitMode {
    Block,
    Delay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElevationRisk {
    Elevated,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevationAction {
    pub requesting_app: String,
    pub description: String,
    pub risk: ElevationRisk,
    pub duration_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElevationResponse {
    Password(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub uid: u32,
    pub user_name: String,
    pub seat_id: String,
    pub session_type: SessionType,
    pub state: String,
    pub locked: bool,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InhibitorInfo {
    pub id: u64,
    pub what: InhibitWhat,
    pub who: String,
    pub why: String,
    pub mode: InhibitMode,
}

// ---- the three top-level wire enums -----------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    CreateSession { user_name: String, seat_id: Option<String>, session_type: Option<String> },
    TerminateSession { session_id: SessionId },
    RegisterCompositor { session_id: SessionId },
    ListSessions,
    SessionStatus { session_id: SessionId },
    LockSession { session_id: SessionId },
    Unlock { session_id: SessionId, user_name: String, password: String },
    ReportActivity { seat_id: String },
    SwitchSession { seat_id: String, session_id: SessionId },
    Inhibit { what: InhibitWhat, who: String, why: String, mode: InhibitMode },
    ReleaseInhibit { inhibit_id: u64 },
    ListInhibitors,
    Suspend,
    Reboot,
    PowerOff,
    RequestElevation { session_id: SessionId, action: ElevationAction },
    RespondElevation { request_id: ElevationRequestId, response: ElevationResponse },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Sessions(Vec<SessionInfo>),
    Session(SessionInfo),
    AuthResult(AuthOutcome),
    InhibitGranted { inhibit_id: u64 },
    Inhibitors(Vec<InhibitorInfo>),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    ShowLockScreen { session_id: SessionId, reason: LockReason },
    HideLockScreen { session_id: SessionId },
    AuthFeedback { session_id: SessionId, outcome: AuthOutcome },
    Dim { seat_id: String },
    Undim { seat_id: String },
    PrepareForSleep,
    ResumedFromSleep,
    SessionActivated { seat_id: String, session_id: SessionId },
    ShowElevationPrompt { request_id: ElevationRequestId, session_id: SessionId, action: ElevationAction },
    ElevationFeedback { request_id: ElevationRequestId, outcome: AuthOutcome },
    HideElevationPrompt { request_id: ElevationRequestId },
}

/// Everything mitos-session ever sends *to* a client is one of these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Response(Response),
    Event(Event),
}

// ---- framing: 4-byte little-endian length, then bincode payload -------
//
// Async mirror of mitos-session's blocking `protocol::{read_message,
// write_message}` -- same wire format, tokio I/O instead of std, because
// mitos-power is async throughout.

pub async fn write_message<W, T>(writer: &mut W, msg: &T) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
    T: Serialize,
{
    let payload = bincode::serialize(msg).map_err(|e| PowerError::Internal(format!("encoding mitos-session message: {e}")))?;
    let len = u32::try_from(payload.len()).map_err(|_| PowerError::Internal("mitos-session message too large to frame".into()))?;
    if len > MAX_MESSAGE_LEN {
        return Err(PowerError::Internal(format!("mitos-session message of {len} bytes exceeds the {MAX_MESSAGE_LEN} byte limit")));
    }
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_message<R, T>(reader: &mut R) -> Result<T>
where
    R: AsyncReadExt + Unpin,
    T: serde::de::DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            PowerError::Hardware("mitos-session closed the connection".into())
        } else {
            PowerError::Io(e)
        }
    })?;
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_MESSAGE_LEN {
        return Err(PowerError::Internal(format!("mitos-session message of {len} bytes exceeds the {MAX_MESSAGE_LEN} byte limit")));
    }
    let mut payload = vec![0u8; len as usize];
    reader.read_exact(&mut payload).await?;
    bincode::deserialize(&payload).map_err(|e| PowerError::Internal(format!("decoding mitos-session message: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    // These verify this mirror is *internally* consistent -- that it
    // round-trips with itself and that the framing is what it claims to
    // be. They can NOT prove the mirror matches mitos-session's real
    // wire format (that would need a real mitos-session to talk to);
    // see the module docs and audit.md.

    #[tokio::test]
    async fn request_round_trips_through_framing() {
        let (mut a, mut b) = tokio::io::duplex(4096);
        write_message(&mut a, &Request::TerminateSession { session_id: 7 }).await.unwrap();
        let got: Request = read_message(&mut b).await.unwrap();
        match got {
            Request::TerminateSession { session_id } => assert_eq!(session_id, 7),
            other => panic!("expected TerminateSession, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn framing_is_four_byte_little_endian_length_then_payload() {
        let (mut a, mut b) = tokio::io::duplex(4096);
        write_message(&mut a, &Request::ListSessions).await.unwrap();
        drop(a);

        let mut raw = Vec::new();
        b.read_to_end(&mut raw).await.unwrap();
        let declared = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
        assert_eq!(declared, raw.len() - 4, "length prefix must equal the payload byte count");
    }

    #[test]
    fn unit_variant_ordinals_match_declaration_order() {
        // bincode encodes an enum's variant as a u32 ordinal (little
        // endian) at the start of the payload. Pin the ordinals of the
        // three requests mitos-power actually sends so an accidental
        // reordering of `Request` above fails a test instead of silently
        // sending the wrong request to mitos-session.
        let ordinal = |r: &Request| {
            let bytes = bincode::serialize(r).unwrap();
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        };
        assert_eq!(ordinal(&Request::TerminateSession { session_id: 0 }), 1);
        assert_eq!(ordinal(&Request::ListSessions), 3);
        assert_eq!(ordinal(&Request::LockSession { session_id: 0 }), 5);
    }

    #[test]
    fn response_ordinals_match_declaration_order() {
        let ordinal = |r: &Response| {
            let bytes = bincode::serialize(r).unwrap();
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        };
        assert_eq!(ordinal(&Response::Ok), 0);
        assert_eq!(ordinal(&Response::Sessions(vec![])), 1);
        assert_eq!(ordinal(&Response::Error(String::new())), 6);
    }

    #[tokio::test]
    async fn oversized_length_prefix_is_rejected_without_allocating() {
        let (mut a, mut b) = tokio::io::duplex(64);
        a.write_all(&(MAX_MESSAGE_LEN + 1).to_le_bytes()).await.unwrap();
        let result: Result<Response> = read_message(&mut b).await;
        assert!(result.is_err());
    }
}
