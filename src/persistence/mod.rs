//! File-backed persistence for settings/statistics/history that should
//! survive a daemon restart. Deliberately dependency-light: each "table"
//! is one JSON file under the configured state directory rather than an
//! embedded SQL database -- the data volume here (settings, capped
//! history, daily-aggregated stats) doesn't need one.
//!
//! NOTE: this module is implemented and unit-tested standalone, but is
//! **not yet called** from `PowerManager`/`Daemon` startup -- nothing
//! currently persists across a restart in practice. Wiring it in (load on
//! `Daemon::new`, save on the relevant `PowerManager` state changes) is
//! the natural next step. See audit.md.

pub mod database;
pub mod history;
pub mod settings;
pub mod statistics;

pub use database::Database;
