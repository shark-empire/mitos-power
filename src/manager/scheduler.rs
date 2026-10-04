//! Delayed shutdown/reboot. Each scheduled operation is a spawned tokio
//! task sleeping until its target time; cancelling aborts the task. A
//! `std::sync::Mutex` is fine here since the critical sections are just
//! HashMap inserts/removes, never held across an `.await`.
//!
//! Deliberately calls the `shutdown`/`reboot` free functions directly
//! rather than going back through `PowerManager::shutdown`/`reboot` --
//! doing that cleanly would need `Scheduler` to hold a way back to its own
//! owning `PowerManager` (a `Weak<PowerManager>` wired up after
//! construction), which is more machinery than this warrants. One
//! consequence worth knowing: **a scheduled shutdown/reboot does not
//! check Shutdown inhibitors** the way an immediate one does -- arguably
//! reasonable (the point of scheduling one is "in N minutes, regardless"),
//! but worth knowing if that surprises you. It *does* still wind sessions
//! down via mitos-session first, same as an immediate one.

use crate::config::GeneralConfig;
use crate::errors::{PowerError, Result};
use crate::shutdown::transition::TransitionBackend;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Default)]
pub struct Scheduler {
    tasks: Mutex<HashMap<Uuid, JoinHandle<()>>>,
}

impl Scheduler {
    pub fn schedule(&self, at: DateTime<Utc>, reboot: bool, backend: TransitionBackend, general: GeneralConfig) -> Uuid {
        let id = Uuid::new_v4();
        let delay = (at - Utc::now()).to_std().unwrap_or(std::time::Duration::ZERO);

        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let requester = "scheduler";
            let session_socket = PathBuf::from(&general.mitos_session_socket_path);
            let _ = crate::session_client::terminate_all_sessions(&session_socket).await;
            let result = if reboot {
                crate::shutdown::reboot::reboot(requester, false, backend).await
            } else {
                crate::shutdown::shutdown::shutdown(requester, false, backend).await
            };
            if let Err(e) = result {
                tracing::error!("scheduled {} failed: {e}", if reboot { "reboot" } else { "shutdown" });
            }
        });

        self.tasks.lock().expect("scheduler mutex poisoned").insert(id, handle);
        id
    }

    pub fn cancel(&self, id: Uuid) -> Result<()> {
        let mut tasks = self.tasks.lock().expect("scheduler mutex poisoned");
        match tasks.remove(&id) {
            Some(handle) => {
                handle.abort();
                Ok(())
            }
            None => Err(PowerError::NotFound(format!("scheduled operation {id}"))),
        }
    }
}
