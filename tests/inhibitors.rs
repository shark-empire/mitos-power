//! End-to-end inhibitor acquire/list/release through the public API,
//! including the `InhibitWhat::All` catch-all (src/inhibitor/manager.rs
//! has the rest of the unit tests).

use mitos_power::inhibitor::{InhibitMode, InhibitWhat, InhibitorManager};
use uuid::Uuid;

#[tokio::test]
async fn all_inhibitor_blocks_every_category() {
    let manager = InhibitorManager::default();
    let client = Uuid::new_v4();
    manager.acquire(client, "mitos-installer".into(), "System update running".into(), InhibitWhat::All, InhibitMode::Block).await;

    assert!(!manager.blockers(InhibitWhat::Suspend).await.is_empty());
    assert!(!manager.blockers(InhibitWhat::Shutdown).await.is_empty());
    assert!(!manager.blockers(InhibitWhat::Idle).await.is_empty());
}

#[tokio::test]
async fn a_specific_inhibitor_does_not_block_other_categories() {
    let manager = InhibitorManager::default();
    let client = Uuid::new_v4();
    manager.acquire(client, "mitos-video".into(), "Movie is playing".into(), InhibitWhat::Idle, InhibitMode::Block).await;

    assert!(!manager.blockers(InhibitWhat::Idle).await.is_empty());
    assert!(manager.blockers(InhibitWhat::Shutdown).await.is_empty());
}
