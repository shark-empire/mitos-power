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
    /// proceed.
    pub async fn blockers(&self, target: InhibitWhat) -> Vec<String> {
        self.inner
            .read()
            .await
            .values()
            .filter(|i| i.what.covers(target))
            .map(|i| format!("{}: {}", i.who, i.why))
            .collect()
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
}
