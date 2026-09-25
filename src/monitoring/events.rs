//! In-memory ring buffer of recent internal daemon events -- distinct from
//! the IPC event bus (`ipc::events`) and the durable audit log
//! (`logging::audit`). Meant for a future "what has this daemon been
//! doing" diagnostic dump.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const CAPACITY: usize = 200;

pub struct RecentEvent {
    pub at: Instant,
    pub description: String,
}

#[derive(Default)]
pub struct EventLog {
    inner: Mutex<VecDeque<RecentEvent>>,
}

impl EventLog {
    pub fn record(&self, description: impl Into<String>) {
        let mut log = self.inner.lock().expect("event log mutex poisoned");
        log.push_back(RecentEvent { at: Instant::now(), description: description.into() });
        while log.len() > CAPACITY {
            log.pop_front();
        }
    }

    pub fn recent(&self, since: Duration) -> Vec<String> {
        let log = self.inner.lock().expect("event log mutex poisoned");
        let now = Instant::now();
        log.iter().filter(|e| now.duration_since(e.at) <= since).map(|e| e.description.clone()).collect()
    }
}
