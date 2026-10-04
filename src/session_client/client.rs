//! Async client for mitos-session's IPC socket.
//!
//! mitos-power needs exactly three things from mitos-session:
//!
//! 1. **End a user's sessions** -- the real implementation behind the
//!    `Logout` IPC method (`shutdown::logout`).
//! 2. **Lock every session before a sleep that mitos-session didn't
//!    itself initiate** -- an automatic hibernate on critical battery, a
//!    lid close, a power button, `mitos-powerctl suspend`. Without this
//!    the desktop would be sitting unlocked on resume.
//! 3. **Wind sessions down before a graceful shutdown/reboot that
//!    mitos-session didn't itself initiate**, so applications get a
//!    chance to exit before the kernel transition (the spec's
//!    "Applications -> mitos-session -> ..." step).
//!
//! Everything here is **strictly best-effort by design**. A minimal
//! MITOS boot with no session manager is a legitimate configuration
//! (mitos-session's own README describes one), so "mitos-session isn't
//! there" is reported as `SessionOpOutcome::Unreachable`, never as an
//! error the caller has to special-case -- and no failure in this
//! module can ever block or veto a suspend/shutdown.
//!
//! A `SessionClient` is single-use after any transport error: once a
//! timeout or I/O error has left the byte stream in an unknown state,
//! further calls fail immediately instead of risking reading half of one
//! reply as the start of the next.

use super::protocol::{read_message, write_message, Message, Request, Response, SessionId, SessionInfo};
use crate::errors::{PowerError, Result};
use std::path::Path;
use std::time::Duration;
use tokio::net::UnixStream;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Upper bound on one request/response round trip. mitos-session's
/// `TerminateSession` is a SIGKILL plus a reap and `LockSession` is a
/// state change plus one event send, so real calls finish in
/// milliseconds; this only exists so a wedged mitos-session can never
/// stall a suspend or shutdown.
const CALL_TIMEOUT: Duration = Duration::from_secs(5);

pub struct SessionClient {
    stream: UnixStream,
    broken: bool,
}

impl SessionClient {
    pub async fn connect(socket_path: &Path) -> Result<Self> {
        let stream = tokio::time::timeout(CONNECT_TIMEOUT, UnixStream::connect(socket_path))
            .await
            .map_err(|_| PowerError::Hardware(format!("timed out connecting to mitos-session at {}", socket_path.display())))?
            .map_err(|e| PowerError::Hardware(format!("could not connect to mitos-session at {}: {e}", socket_path.display())))?;
        Ok(Self { stream, broken: false })
    }

    /// One request, one response. An `Err` here is always a *transport*
    /// problem (and marks the client broken); an application-level
    /// refusal from mitos-session arrives as `Ok(Response::Error(..))`.
    async fn call(&mut self, request: Request) -> Result<Response> {
        if self.broken {
            return Err(PowerError::Hardware("mitos-session connection is no longer usable".into()));
        }
        let outcome = tokio::time::timeout(CALL_TIMEOUT, self.exchange(request)).await;
        match outcome {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => {
                self.broken = true;
                Err(e)
            }
            Err(_) => {
                self.broken = true;
                Err(PowerError::Hardware("timed out waiting for mitos-session to answer".into()))
            }
        }
    }

    async fn exchange(&mut self, request: Request) -> Result<Response> {
        write_message(&mut self.stream, &request).await?;
        loop {
            match read_message::<_, Message>(&mut self.stream).await? {
                Message::Response(response) => return Ok(response),
                // This connection never registers as a compositor, so
                // mitos-session shouldn't push events to it; skip
                // defensively rather than mistaking one for our reply.
                Message::Event(_) => continue,
            }
        }
    }

    pub async fn list_sessions(&mut self) -> Result<Vec<SessionInfo>> {
        match self.call(Request::ListSessions).await? {
            Response::Sessions(sessions) => Ok(sessions),
            Response::Error(e) => Err(PowerError::Hardware(format!("mitos-session: {e}"))),
            other => Err(PowerError::Internal(format!("mitos-session: unexpected reply to ListSessions: {other:?}"))),
        }
    }

    pub async fn lock_session(&mut self, session_id: SessionId) -> Result<()> {
        self.expect_ok(Request::LockSession { session_id }).await
    }

    pub async fn terminate_session(&mut self, session_id: SessionId) -> Result<()> {
        self.expect_ok(Request::TerminateSession { session_id }).await
    }

    async fn expect_ok(&mut self, request: Request) -> Result<()> {
        match self.call(request).await? {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(PowerError::Hardware(format!("mitos-session: {e}"))),
            other => Err(PowerError::Internal(format!("mitos-session: unexpected reply: {other:?}"))),
        }
    }
}

/// What happened when mitos-power tried to act on mitos-session's
/// sessions. Deliberately not a `Result`: none of these outcomes is a
/// reason to block the power action that prompted the attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOpOutcome {
    /// Couldn't connect at all -- no session manager running. Normal on
    /// a minimal boot.
    Unreachable,
    /// Connected, but couldn't get a usable session list back.
    Failed,
    /// Connected and listed, but no session needed the operation.
    NothingToDo,
    /// `ok` sessions handled; `failed` sessions were refused or errored.
    Done { ok: usize, failed: usize },
}

#[derive(Clone, Copy)]
enum SessionOp {
    Lock,
    Terminate,
}

/// Lock every session that isn't already locked.
///
/// A refusal is logged at `warn`, not `debug`: a session that could not
/// be locked before sleep will be sitting unlocked on resume, which an
/// operator should be able to see. (mitos-session refuses to lock a user
/// with no password set, or while an application holds a lock-screen
/// inhibitor -- both are legitimate, and neither blocks the suspend.)
pub async fn lock_all_sessions(socket_path: &Path) -> SessionOpOutcome {
    apply_to_sessions(socket_path, None, SessionOp::Lock).await
}

/// End every session belonging to `uid`.
pub async fn terminate_sessions_for_uid(socket_path: &Path, uid: u32) -> SessionOpOutcome {
    apply_to_sessions(socket_path, Some(uid), SessionOp::Terminate).await
}

/// End every session, for every user -- the "wind down before a
/// graceful shutdown/reboot" step.
pub async fn terminate_all_sessions(socket_path: &Path) -> SessionOpOutcome {
    apply_to_sessions(socket_path, None, SessionOp::Terminate).await
}

async fn apply_to_sessions(socket_path: &Path, only_uid: Option<u32>, op: SessionOp) -> SessionOpOutcome {
    let mut client = match SessionClient::connect(socket_path).await {
        Ok(client) => client,
        Err(e) => {
            tracing::debug!("mitos-session not reachable, skipping session operation: {e}");
            return SessionOpOutcome::Unreachable;
        }
    };

    let sessions = match client.list_sessions().await {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::warn!("connected to mitos-session but could not list sessions: {e}");
            return SessionOpOutcome::Failed;
        }
    };

    let targets: Vec<&SessionInfo> = sessions
        .iter()
        .filter(|s| only_uid.map_or(true, |uid| s.uid == uid))
        // Re-locking a locked session would make mitos-session re-send
        // ShowLockScreen (with reason "Manual") to its compositor -- pure
        // noise, and it would overwrite the "Suspend" reason if
        // mitos-session had just locked the session itself.
        .filter(|s| !(matches!(op, SessionOp::Lock) && s.locked))
        .collect();

    if targets.is_empty() {
        return SessionOpOutcome::NothingToDo;
    }

    let (mut ok, mut failed) = (0usize, 0usize);
    for session in targets {
        let result = match op {
            SessionOp::Lock => client.lock_session(session.id).await,
            SessionOp::Terminate => client.terminate_session(session.id).await,
        };
        match result {
            Ok(()) => ok += 1,
            Err(e) => {
                failed += 1;
                let what = match op {
                    SessionOp::Lock => "lock",
                    SessionOp::Terminate => "terminate",
                };
                tracing::warn!("could not {what} session {} ({}): {e}", session.id, session.user_name);
            }
        }
    }
    SessionOpOutcome::Done { ok, failed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_client::protocol::SessionType;
    use std::time::SystemTime;
    use tokio::net::UnixListener;

    fn session(id: SessionId, uid: u32, locked: bool) -> SessionInfo {
        SessionInfo {
            id,
            uid,
            user_name: format!("user{uid}"),
            seat_id: "seat0".into(),
            session_type: SessionType::Wayland,
            state: "active".into(),
            locked,
            created_at: SystemTime::UNIX_EPOCH,
        }
    }

    /// A stand-in mitos-session: answers `ListSessions` with `sessions`
    /// and records every LockSession/TerminateSession id it receives.
    /// It speaks the protocol via *this crate's mirror*, so these tests
    /// exercise mitos-power's client logic, not fidelity to the real
    /// mitos-session wire format (which needs the real daemon to test).
    async fn fake_session_manager(
        socket: std::path::PathBuf,
        sessions: Vec<SessionInfo>,
        reject_lock: bool,
    ) -> std::sync::Arc<tokio::sync::Mutex<Vec<(String, SessionId)>>> {
        let log = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let listener = UnixListener::bind(&socket).unwrap();
        let log_for_task = log.clone();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            loop {
                let request: Request = match read_message(&mut stream).await {
                    Ok(r) => r,
                    Err(_) => return, // client hung up
                };
                let reply = match request {
                    Request::ListSessions => Response::Sessions(sessions.clone()),
                    Request::LockSession { session_id } => {
                        log_for_task.lock().await.push(("lock".into(), session_id));
                        if reject_lock {
                            Response::Error("Lock screen disabled: No password set for this user.".into())
                        } else {
                            Response::Ok
                        }
                    }
                    Request::TerminateSession { session_id } => {
                        log_for_task.lock().await.push(("terminate".into(), session_id));
                        Response::Ok
                    }
                    other => Response::Error(format!("unexpected request in test: {other:?}")),
                };
                if write_message(&mut stream, &Message::Response(reply)).await.is_err() {
                    return;
                }
            }
        });
        log
    }

    fn temp_socket(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mitos-power-session-client-{tag}-{}-{}", std::process::id(), uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("session.sock")
    }

    #[tokio::test]
    async fn unreachable_when_no_session_manager_is_listening() {
        let socket = temp_socket("absent"); // never bound
        assert_eq!(lock_all_sessions(&socket).await, SessionOpOutcome::Unreachable);
        assert_eq!(terminate_sessions_for_uid(&socket, 1000).await, SessionOpOutcome::Unreachable);
        assert_eq!(terminate_all_sessions(&socket).await, SessionOpOutcome::Unreachable);
    }

    #[tokio::test]
    async fn terminate_for_uid_only_touches_that_users_sessions() {
        let socket = temp_socket("terminate");
        let log = fake_session_manager(socket.clone(), vec![session(1, 1000, false), session(2, 1001, false), session(3, 1000, false)], false).await;

        let outcome = terminate_sessions_for_uid(&socket, 1000).await;
        assert_eq!(outcome, SessionOpOutcome::Done { ok: 2, failed: 0 });

        let seen = log.lock().await.clone();
        assert_eq!(seen, vec![("terminate".to_string(), 1), ("terminate".to_string(), 3)]);
    }

    #[tokio::test]
    async fn no_matching_uid_is_nothing_to_do_not_an_error() {
        let socket = temp_socket("nomatch");
        let _log = fake_session_manager(socket.clone(), vec![session(1, 1000, false)], false).await;
        assert_eq!(terminate_sessions_for_uid(&socket, 4242).await, SessionOpOutcome::NothingToDo);
    }

    #[tokio::test]
    async fn lock_all_skips_sessions_that_are_already_locked() {
        let socket = temp_socket("lock");
        let log = fake_session_manager(socket.clone(), vec![session(1, 1000, true), session(2, 1001, false)], false).await;

        assert_eq!(lock_all_sessions(&socket).await, SessionOpOutcome::Done { ok: 1, failed: 0 });
        assert_eq!(log.lock().await.clone(), vec![("lock".to_string(), 2)]);
    }

    #[tokio::test]
    async fn a_refused_lock_is_counted_but_never_an_error() {
        // e.g. mitos-session refuses to lock a user with no password set.
        let socket = temp_socket("refused");
        let _log = fake_session_manager(socket.clone(), vec![session(1, 1000, false)], true).await;
        assert_eq!(lock_all_sessions(&socket).await, SessionOpOutcome::Done { ok: 0, failed: 1 });
    }
}
