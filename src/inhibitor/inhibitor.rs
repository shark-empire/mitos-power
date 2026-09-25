//! A single held inhibitor.

use super::reason::{InhibitMode, InhibitWhat};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Inhibitor {
    pub id: Uuid,
    pub client_id: Uuid,
    pub who: String,
    pub why: String,
    pub what: InhibitWhat,
    pub mode: InhibitMode,
    pub created_at: DateTime<Utc>,
}

/// Wire representation returned by `ListInhibitors`.
#[derive(Debug, Clone, Serialize)]
pub struct InhibitorInfo {
    pub id: Uuid,
    pub who: String,
    pub why: String,
    pub what: InhibitWhat,
    pub mode: InhibitMode,
    pub created_at: DateTime<Utc>,
}

impl From<&Inhibitor> for InhibitorInfo {
    fn from(i: &Inhibitor) -> Self {
        InhibitorInfo { id: i.id, who: i.who.clone(), why: i.why.clone(), what: i.what, mode: i.mode, created_at: i.created_at }
    }
}
