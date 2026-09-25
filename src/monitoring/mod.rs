//! Self-observability: uptime/health, a point-in-time diagnostics snapshot,
//! runtime metrics counters, and a small in-memory recent-events ring
//! buffer. Real and unit-tested, but (like `persistence`) not yet wired
//! into the live request path -- see audit.md.

pub mod diagnostics;
pub mod events;
pub mod health;
pub mod statistics;

pub use diagnostics::Diagnostics;
pub use health::HealthTracker;
