//! Ends the requesting user's sessions without powering the machine off.
//!
//! mitos-power doesn't own sessions -- mitos-session does. `Logout`
//! therefore asks mitos-session (over *its* socket, see `session_client`)
//! to terminate every session belonging to the caller's uid. Previously
//! this was a documented no-op because mitos-session's wire protocol
//! wasn't available to integrate against.
//!
//! Semantics worth knowing:
//! - It ends the **caller's own** sessions. There's no "log out another
//!   user" form; that's not something this method ever accepted a
//!   parameter for.
//! - Unlike the pre-sleep/pre-shutdown session handling (which is
//!   best-effort and never blocks the power action), a `Logout` that
//!   *couldn't* log anyone out reports an error -- the caller asked for
//!   exactly that one thing, so "Ok" would be a lie.

use crate::errors::{PowerError, Result};
use crate::logging::audit;
use crate::session_client::{terminate_sessions_for_uid, SessionOpOutcome};
use std::path::Path;

pub async fn logout(requester_uid: u32, session_socket: &Path) -> Result<()> {
    audit::log_privileged_action(&format!("uid:{requester_uid}"), "logout", false);
    let outcome = terminate_sessions_for_uid(session_socket, requester_uid).await;
    outcome_to_result(outcome, requester_uid, session_socket)
}

fn outcome_to_result(outcome: SessionOpOutcome, uid: u32, socket: &Path) -> Result<()> {
    match outcome {
        SessionOpOutcome::Done { ok, failed } if ok > 0 => {
            if failed > 0 {
                tracing::warn!("logout for uid {uid}: ended {ok} session(s), but mitos-session refused to end {failed} more");
            } else {
                tracing::info!("logout for uid {uid}: ended {ok} session(s)");
            }
            Ok(())
        }
        SessionOpOutcome::Done { failed, .. } => {
            Err(PowerError::Hardware(format!("mitos-session refused to end {failed} session(s) for uid {uid}")))
        }
        SessionOpOutcome::NothingToDo => Err(PowerError::NotFound(format!("mitos-session has no session for uid {uid}"))),
        SessionOpOutcome::Unreachable => Err(PowerError::Hardware(format!(
            "mitos-session is not reachable at {} -- there is no session manager to log out of",
            socket.display()
        ))),
        SessionOpOutcome::Failed => Err(PowerError::Hardware("mitos-session did not return a usable session list".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn socket() -> &'static Path {
        Path::new("/run/mitos-session/session.sock")
    }

    #[test]
    fn ending_at_least_one_session_is_success_even_if_another_was_refused() {
        assert!(outcome_to_result(SessionOpOutcome::Done { ok: 1, failed: 1 }, 1000, socket()).is_ok());
    }

    #[test]
    fn every_session_refused_is_an_error() {
        assert!(matches!(outcome_to_result(SessionOpOutcome::Done { ok: 0, failed: 2 }, 1000, socket()), Err(PowerError::Hardware(_))));
    }

    #[test]
    fn no_session_for_the_caller_is_not_found() {
        assert!(matches!(outcome_to_result(SessionOpOutcome::NothingToDo, 1000, socket()), Err(PowerError::NotFound(_))));
    }

    #[test]
    fn unreachable_session_manager_is_reported_not_swallowed() {
        assert!(matches!(outcome_to_result(SessionOpOutcome::Unreachable, 1000, socket()), Err(PowerError::Hardware(_))));
    }

    #[tokio::test]
    async fn logout_without_a_session_manager_fails_cleanly() {
        let missing = std::env::temp_dir().join(format!("mitos-power-no-session-{}", uuid::Uuid::new_v4())).join("session.sock");
        assert!(logout(1000, &missing).await.is_err());
    }
}
