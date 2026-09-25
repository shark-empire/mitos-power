//! Top-level daemon: construct everything from config, then run the IPC
//! server and the hardware event loop side by side until a shutdown signal
//! arrives.

use crate::config::Config;
use crate::daemon::lifecycle::{self, LifecycleEvent};
use crate::errors::Result;
use crate::ipc::IpcServer;
use crate::manager::PowerManager;
use std::path::PathBuf;
use std::sync::Arc;

pub struct Daemon {
    manager: Arc<PowerManager>,
    config_dir: PathBuf,
}

impl Daemon {
    pub fn new(config: Config, config_dir: PathBuf) -> Result<Self> {
        let manager = Arc::new(PowerManager::new(config)?);
        Ok(Self { manager, config_dir })
    }

    pub async fn run(self) -> Result<()> {
        let (socket_path, socket_group) = {
            let config = self.manager.config.read().await;
            (PathBuf::from(&config.general.socket_path), config.general.socket_group.clone())
        };

        let ipc_server = IpcServer::new(socket_path, socket_group, self.manager.clone());
        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

        let event_loop_manager = self.manager.clone();
        let event_loop_handle = tokio::spawn(async move {
            crate::daemon::event_loop::run(event_loop_manager, shutdown_rx).await;
        });

        tracing::info!("mitos-power daemon starting (config: {})", self.config_dir.display());

        tokio::select! {
            result = ipc_server.run() => {
                if let Err(e) = &result {
                    tracing::error!("IPC server exited: {e}");
                }
                result?;
            }
            () = self.signal_loop() => {}
        }

        let _ = shutdown_tx.send(());
        let _ = event_loop_handle.await;
        tracing::info!("mitos-power daemon stopped");
        Ok(())
    }

    /// Handles signals for the lifetime of the daemon. SIGTERM/SIGINT
    /// return (letting `run()`'s select! fall through and start shutdown).
    /// SIGHUP reloads config into the shared `RwLock<Config>` in place and
    /// keeps looping -- NOTE this updates every field readers fetch fresh
    /// each time (security.privileged_group, sleep timing, battery/thermal
    /// thresholds read via `manager.config`), but does *not* rebuild
    /// subsystems that cached values at construction time (the profile
    /// list, the idle detector's timeouts). A full reload of those still
    /// needs a daemon restart -- see docs/architecture.md "Config reload".
    async fn signal_loop(&self) {
        loop {
            match lifecycle::wait_for_signal().await {
                LifecycleEvent::Shutdown => {
                    tracing::info!("received shutdown signal");
                    return;
                }
                LifecycleEvent::ReloadConfig => {
                    tracing::info!("received SIGHUP, reloading config from {}", self.config_dir.display());
                    match Config::load(&self.config_dir) {
                        Ok(new_config) => {
                            *self.manager.config.write().await = new_config;
                            tracing::info!("config reloaded");
                        }
                        Err(e) => tracing::error!("config reload failed, keeping previous config: {e}"),
                    }
                }
            }
        }
    }
}
