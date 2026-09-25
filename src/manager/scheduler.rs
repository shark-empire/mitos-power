//! Delayed shutdown/reboot. Each scheduled operation is a spawned tokio
//! task sleeping until its target time; cancelling aborts the task. A
//! `std::sync::Mutex` is fine here since the critical sections are just
//! HashMap inserts/removes, never held across an `.await`.

use crate::errors::{PowerError, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Default)]
pub struct Scheduler {
    tasks: Mutex<HashMap<Uuid, JoinHandle<()>>>,
}

impl Scheduler {
    pub fn schedule(&self, at: DateTime<Utc>, reboot: bool) -> Uuid {
        let id = Uuid::new_v4();
        let delay = (at - Utc::now()).to_std().unwrap_or(std::time::Duration::ZERO);

        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let requester = "scheduler";
            let result = if reboot {
                crate::shutdown::reboot::reboot(requester, false).await
            } else {
                crate::shutdown::shutdown::shutdown(requester, false).await
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
