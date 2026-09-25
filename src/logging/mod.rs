//! Logging setup. `audit` is the structured trail of privileged actions
//! (suspend/shutdown/reboot -- who and when), kept separate from normal
//! application logging.

pub mod audit;

/// Initializes the global tracing subscriber. Respects `RUST_LOG` (e.g.
/// `RUST_LOG=mitos_power=debug`); defaults to `info`.
pub fn init() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_target(true).init();
}
