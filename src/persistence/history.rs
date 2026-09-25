//! Capped rolling history of notable events (profile changes, suspend/
//! resume, AC plug/unplug) for casual "what happened recently" queries.
//! Not a full audit trail -- see `logging::audit` for that.

use super::database::Database;
use crate::errors::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_ENTRIES: usize = 500;
const TABLE: &str = "history";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    pub entries: VecDeque<HistoryEntry>,
}

pub fn append(db: &Database, kind: &str, detail: &str) -> Result<()> {
    let mut history: History = db.load(TABLE)?;
    history.entries.push_back(HistoryEntry { at: Utc::now(), kind: kind.into(), detail: detail.into() });
    while history.entries.len() > MAX_ENTRIES {
        history.entries.pop_front();
    }
    db.save(TABLE, &history)
}

pub fn recent(db: &Database, limit: usize) -> Result<Vec<HistoryEntry>> {
    let history: History = db.load(TABLE)?;
    Ok(history.entries.iter().rev().take(limit).cloned().collect())
}
