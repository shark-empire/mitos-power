//! Long-term, daily-aggregated battery statistics -- distinct from the
//! in-memory rolling window in `battery::statistics` (which resets every
//! daemon restart) and from `monitoring::statistics` (runtime daemon
//! metrics, not battery data).

use super::database::Database;
use crate::errors::Result;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBatteryStat {
    pub date: NaiveDate,
    pub min_percent: f32,
    pub max_percent: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedStatistics {
    pub daily: HashMap<String, DailyBatteryStat>, // keyed by ISO date string, e.g. "2026-09-22"
    pub last_updated: Option<DateTime<Utc>>,
}

const TABLE: &str = "statistics";

pub fn load(db: &Database) -> Result<PersistedStatistics> {
    db.load(TABLE)
}

pub fn record_sample(db: &Database, percent: f32) -> Result<()> {
    let mut stats = load(db)?;
    let today = Utc::now().date_naive();
    let key = today.to_string();
    let entry = stats.daily.entry(key).or_insert(DailyBatteryStat { date: today, min_percent: percent, max_percent: percent });
    entry.min_percent = entry.min_percent.min(percent);
    entry.max_percent = entry.max_percent.max(percent);
    stats.last_updated = Some(Utc::now());
    db.save(TABLE, &stats)
}
