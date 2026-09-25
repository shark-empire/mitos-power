//! Runtime daemon metrics (IPC requests served, suspend/resume counts,
//! policy actions executed). In-memory only -- see `persistence` for
//! anything that should survive a restart.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Metrics {
    pub ipc_requests_total: AtomicU64,
    pub suspend_count: AtomicU64,
    pub resume_count: AtomicU64,
    pub policy_actions_executed: AtomicU64,
}

impl Metrics {
    pub fn incr_ipc_requests(&self) {
        self.ipc_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_suspend(&self) {
        self.suspend_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_resume(&self) {
        self.resume_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_policy_actions(&self) {
        self.policy_actions_executed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            ipc_requests_total: self.ipc_requests_total.load(Ordering::Relaxed),
            suspend_count: self.suspend_count.load(Ordering::Relaxed),
            resume_count: self.resume_count.load(Ordering::Relaxed),
            policy_actions_executed: self.policy_actions_executed.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub ipc_requests_total: u64,
    pub suspend_count: u64,
    pub resume_count: u64,
    pub policy_actions_executed: u64,
}
