//! Thread-safe inhibitor registry. One instance lives on `PowerManager` and
//! is shared by every IPC client connection.

use super::inhibitor::{Inhibitor, InhibitorInfo};
use super::reason::InhibitWhat;
use crate::errors::{PowerError, Result};
use chrono::Utc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Default)]
pub struct InhibitorManager {
    inner: RwLock<HashMap<Uuid, Inhibitor>>,
}

impl InhibitorManager {
    pub async fn acquire(
        &self,
        client_id: Uuid,
        who: String,
        why: String,
        what: InhibitWhat,
        mode: super::reason::InhibitMode,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let inhibitor = Inhibitor { id, client_id, who, why, what, mode, created_at: Utc::now() };
        self.inner.write().await.insert(id, inhibitor);
        id
    }

    pub async fn release(&self, id: Uuid) -> Result<()> {
        let mut map = self.inner.write().await;
        map.remove(&id).map(|_| ()).ok_or_else(|| PowerError::NotFound(format!("inhibitor {id}")))
    }

    pub async fn list(&self) -> Vec<InhibitorInfo> {
        self.inner.read().await.values().map(InhibitorInfo::from).collect()
    }

    /// Human-readable "who: why" strings for every currently held inhibitor
    /// that covers `target`. An empty vec means the action is allowed to
    /// proceed. Treats Block and Delay modes identically -- appropriate
    /// for callers (like idle dim/off) where "wait a few seconds, then do
    /// it anyway" doesn't make sense for a recurring action. For
    /// Suspend/Shutdown, prefer `split_blockers` so Delay-mode inhibitors
    /// get their grace period instead of hard-blocking forever.
    pub async fn blockers(&self, target: InhibitWhat) -> Vec<String> {
        self.inner
            .read()
            .await
            .values()
            .filter(|i| i.what.covers(target))
            .map(|i| format!("{}: {}", i.who, i.why))
            .collect()
    }

    /// Like `blockers`, but split by mode: `(block_mode, delay_mode)`.
    /// Callers that want real Delay semantics (a grace period, then
    /// proceed anyway) should check `block_mode` for a hard failure and
    /// use `wait_for_delay_clear` for `delay_mode` -- see
    /// `PowerManager::ensure_not_inhibited`.
    pub async fn split_blockers(&self, target: InhibitWhat) -> (Vec<String>, Vec<String>) {
        let map = self.inner.read().await;
        let mut blocking = Vec::new();
        let mut delaying = Vec::new();
        for i in map.values().filter(|i| i.what.covers(target)) {
            let label = format!("{}: {}", i.who, i.why);
            match i.mode {
                super::reason::InhibitMode::Block => blocking.push(label),
                super::reason::InhibitMode::Delay => delaying.push(label),
            }
        }
        (blocking, delaying)
    }

    /// Polls until no Delay-mode inhibitor covers `target` anymore, or
    /// `grace` elapses, whichever comes first. Returns `true` if it
    /// cleared naturally, `false` if the grace period ran out (the caller
    /// should proceed anyway in that case -- Delay never blocks forever).
    pub async fn wait_for_delay_clear(&self, target: InhibitWhat, grace: std::time::Duration) -> bool {
        let deadline = tokio::time::Instant::now() + grace;
        let poll_interval = std::time::Duration::from_millis(250).min(grace);
        loop {
            let (_, delaying) = self.split_blockers(target).await;
            if delaying.is_empty() {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Called by the IPC server when a client connection closes, so
    /// inhibitors held by a crashed/exited process don't wedge the system
    /// forever.
    pub async fn release_all_for_client(&self, client_id: Uuid) {
        self.inner.write().await.retain(|_, i| i.client_id != client_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inhibitor::reason::InhibitMode;

    #[tokio::test]
    async fn acquire_then_blockers_then_release() {
        let mgr = InhibitorManager::default();
        let client = Uuid::new_v4();
        let id = mgr
            .acquire(client, "mitos-video".into(), "Movie is playing".into(), InhibitWhat::Suspend, InhibitMode::Block)
            .await;

        let blockers = mgr.blockers(InhibitWhat::Suspend).await;
        assert_eq!(blockers, vec!["mitos-video: Movie is playing".to_string()]);
        // "All" is a different InhibitWhat, so it should NOT show up under Idle unless the inhibitor itself is "all".
        assert!(mgr.blockers(InhibitWhat::Idle).await.is_empty());

        mgr.release(id).await.unwrap();
        assert!(mgr.blockers(InhibitWhat::Suspend).await.is_empty());
    }

    #[tokio::test]
    async fn disconnect_releases_that_clients_inhibitors_only() {
        let mgr = InhibitorManager::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        mgr.acquire(a, "app-a".into(), "reason-a".into(), InhibitWhat::Idle, InhibitMode::Block).await;
        mgr.acquire(b, "app-b".into(), "reason-b".into(), InhibitWhat::Idle, InhibitMode::Block).await;

        mgr.release_all_for_client(a).await;

        let remaining = mgr.list().await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].who, "app-b");
    }

    #[tokio::test]
    async fn delay_mode_clears_naturally_before_the_grace_period() {
        let mgr = InhibitorManager::default();
        let client = Uuid::new_v4();
        let id = mgr.acquire(client, "installer".into(), "System update running".into(), InhibitWhat::Shutdown, InhibitMode::Delay).await;

        // Released before we ever start waiting -- wait_for_delay_clear's
        // first check (before its first sleep) should see it's already gone.
        mgr.release(id).await.unwrap();

        let cleared = mgr.wait_for_delay_clear(InhibitWhat::Shutdown, std::time::Duration::from_secs(2)).await;
        assert!(cleared, "should report cleared since the inhibitor was already released before waiting");
    }

    #[tokio::test]
    async fn delay_mode_times_out_and_reports_not_cleared() {
        let mgr = InhibitorManager::default();
        let client = Uuid::new_v4();
        mgr.acquire(client, "installer".into(), "System update running".into(), InhibitWhat::Shutdown, InhibitMode::Delay).await;
        // Never released -- the grace period must still expire and hand
        // control back, not hang forever.
        let cleared = mgr.wait_for_delay_clear(InhibitWhat::Shutdown, std::time::Duration::from_millis(300)).await;
        assert!(!cleared, "should report NOT cleared once the grace period elapses with the inhibitor still held");
    }

    #[tokio::test]
    async fn split_blockers_separates_block_from_delay() {
        let mgr = InhibitorManager::default();
        let client = Uuid::new_v4();
        mgr.acquire(client, "backup".into(), "Backup running".into(), InhibitWhat::Shutdown, InhibitMode::Block).await;
        mgr.acquire(client, "installer".into(), "Update running".into(), InhibitWhat::Shutdown, InhibitMode::Delay).await;

        let (blocking, delaying) = mgr.split_blockers(InhibitWhat::Shutdown).await;
        assert_eq!(blocking.len(), 1);
        assert_eq!(delaying.len(), 1);
        assert!(blocking[0].contains("backup"));
        assert!(delaying[0].contains("installer"));
    }
}
