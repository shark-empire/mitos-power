//! Signal handling.

use tokio::signal::unix::{signal, SignalKind};

pub enum LifecycleEvent {
    Shutdown,
    ReloadConfig,
}

/// Waits for SIGTERM/SIGINT (graceful shutdown) or SIGHUP (config reload).
pub async fn wait_for_signal() -> LifecycleEvent {
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
    let mut sighup = signal(SignalKind::hangup()).expect("failed to install SIGHUP handler");

    tokio::select! {
        _ = sigterm.recv() => LifecycleEvent::Shutdown,
        _ = sigint.recv() => LifecycleEvent::Shutdown,
        _ = sighup.recv() => LifecycleEvent::ReloadConfig,
    }
}
