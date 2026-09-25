//! Unix domain socket server. One task per client connection; each task
//! both services that client's requests and forwards broadcast daemon
//! events the client has subscribed to.

use super::events::DaemonEvent;
use super::messages::{
    AcquireInhibitorParams, CancelScheduledParams, ForceParams, GetBatteryParams, ReleaseInhibitorParams,
    ScheduleShutdownParams, SetBrightnessParams, SetIdleTimeoutParams, SetProfileParams,
};
use super::protocol::{error_response, ok_response, ClientMessage, ServerMessage};
use super::{permissions, protocol};
use crate::errors::{PowerError, Result};
use crate::manager::manager::PowerManager;
use chrono::Utc;
use std::collections::HashSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::UCred;
use tokio::net::{UnixListener, UnixStream};
use uuid::Uuid;

pub struct IpcServer {
    socket_path: PathBuf,
    socket_group: String,
    manager: Arc<PowerManager>,
}

impl IpcServer {
    pub fn new(socket_path: PathBuf, socket_group: String, manager: Arc<PowerManager>) -> Self {
        Self { socket_path, socket_group, manager }
    }

    pub async fn run(&self) -> Result<()> {
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        set_socket_permissions(&self.socket_path, &self.socket_group);
        tracing::info!("IPC listening on {}", self.socket_path.display());

        loop {
            let (stream, _addr) = listener.accept().await?;
            let manager = self.manager.clone();
            tokio::spawn(async move {
                let client_id = Uuid::new_v4();
                if let Err(e) = handle_client(stream, manager.clone(), client_id).await {
                    tracing::debug!("client {client_id} connection ended: {e}");
                }
                manager.inhibitors.release_all_for_client(client_id).await;
            });
        }
    }
}

fn set_socket_permissions(path: &Path, group: &str) {
    // Best-effort: group ownership requires the daemon to run as root (it
    // does, for suspend/reboot anyway), and NSS lookups can fail on
    // minimal/early-boot systems -- neither should stop the socket working
    // for root and members of the group by fallback mode bits.
    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660)) {
        tracing::warn!("could not set socket permissions on {}: {e}", path.display());
    }
    match nix::unistd::Group::from_name(group) {
        Ok(Some(g)) => {
            let _ = nix::unistd::chown(path, None, Some(g.gid));
        }
        _ => tracing::warn!("group '{group}' not found; socket left at default ownership"),
    }
}

async fn handle_client(stream: UnixStream, manager: Arc<PowerManager>, client_id: Uuid) -> Result<()> {
    let cred = stream.peer_cred()?;
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let mut events_rx = manager.events_tx.subscribe();
    let mut subscribed: HashSet<String> = HashSet::new();

    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Some(line) = line? else { break };
                if line.trim().is_empty() {
                    continue;
                }
                let client_msg: ClientMessage = match serde_json::from_str(&line) {
                    Ok(m) => m,
                    Err(e) => {
                        write_line(&mut writer, &error_response("", "INVALID_REQUEST", &e.to_string())).await?;
                        continue;
                    }
                };
                match client_msg {
                    ClientMessage::Request { id, method, params } => {
                        let resp = dispatch(&manager, &cred, client_id, &id, &method, &params).await;
                        write_line(&mut writer, &resp).await?;
                    }
                    ClientMessage::Subscribe { events } => {
                        subscribed.extend(events);
                    }
                    ClientMessage::Unsubscribe { events } => {
                        for e in events {
                            subscribed.remove(&e);
                        }
                    }
                }
            }
            event = events_rx.recv() => {
                let event: DaemonEvent = match event {
                    Ok(e) => e,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("client {client_id} lagged, dropped {n} events");
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                if subscribed.contains(event.name) || subscribed.contains("*") {
                    let msg = ServerMessage::Event { event: event.name.to_string(), data: event.data, timestamp: Utc::now() };
                    write_line(&mut writer, &msg).await?;
                }
            }
        }
    }

    Ok(())
}

async fn write_line(writer: &mut (impl AsyncWriteExt + Unpin), msg: &ServerMessage) -> Result<()> {
    let mut line = serde_json::to_string(msg)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    Ok(())
}

fn requester(cred: &UCred) -> String {
    format!("uid:{}", cred.uid())
}

fn parse_params<T: serde::de::DeserializeOwned>(value: &serde_json::Value) -> Result<T> {
    serde_json::from_value(value.clone()).map_err(|e| PowerError::InvalidParams(e.to_string()))
}

async fn dispatch(
    manager: &PowerManager,
    cred: &UCred,
    client_id: Uuid,
    id: &str,
    method: &str,
    params: &serde_json::Value,
) -> ServerMessage {
    match dispatch_inner(manager, cred, client_id, method, params).await {
        Ok(value) => ok_response(id, value),
        Err(e) => error_response(id, error_code(&e), &e.to_string()),
    }
}

async fn dispatch_inner(
    manager: &PowerManager,
    cred: &UCred,
    client_id: Uuid,
    method: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    let privileged_group = manager.security_config().await.privileged_group;
    permissions::check(method, cred, &privileged_group)?;

    Ok(match method {
        "Ping" => serde_json::json!({ "pong": true }),
        "GetVersion" => serde_json::json!({ "version": env!("CARGO_PKG_VERSION") }),

        "GetPowerState" => serde_json::to_value(manager.get_power_state().await)?,
        "GetBattery" => {
            let p: GetBatteryParams = if params.is_null() { GetBatteryParams::default() } else { parse_params(params)? };
            serde_json::to_value(manager.get_battery(p.id.as_deref()).await?)?
        }
        "GetBatteries" => serde_json::to_value(manager.get_batteries().await)?,
        "GetAcState" => serde_json::to_value(manager.get_ac_state().await)?,

        "GetProfile" => serde_json::to_value(manager.get_profile().await)?,
        "ListProfiles" => serde_json::to_value(manager.list_profiles().await)?,
        "SetProfile" => {
            let p: SetProfileParams = parse_params(params)?;
            manager.set_profile(&p.name).await?;
            serde_json::json!({})
        }

        "GetBrightness" => serde_json::json!({ "percent": manager.get_brightness().await? }),
        "SetBrightness" => {
            let p: SetBrightnessParams = parse_params(params)?;
            manager.set_brightness(p.percent).await?;
            serde_json::json!({})
        }

        "Suspend" => { manager.suspend(&requester(cred)).await?; serde_json::json!({}) }
        "Hibernate" => { manager.hibernate(&requester(cred)).await?; serde_json::json!({}) }
        "HybridSleep" => { manager.hybrid_sleep(&requester(cred)).await?; serde_json::json!({}) }
        "SuspendThenHibernate" => { manager.suspend_then_hibernate(&requester(cred)).await?; serde_json::json!({}) }

        "Shutdown" => {
            let p: ForceParams = if params.is_null() { ForceParams::default() } else { parse_params(params)? };
            manager.shutdown(&requester(cred), p.force).await?;
            serde_json::json!({})
        }
        "Reboot" => {
            let p: ForceParams = if params.is_null() { ForceParams::default() } else { parse_params(params)? };
            manager.reboot(&requester(cred), p.force).await?;
            serde_json::json!({})
        }
        "Poweroff" => { manager.poweroff(&requester(cred)).await?; serde_json::json!({}) }
        "Logout" => { manager.logout(&requester(cred)).await?; serde_json::json!({}) }

        "ScheduleShutdown" => {
            let p: ScheduleShutdownParams = parse_params(params)?;
            let id = manager.schedule_shutdown(p.at, p.reboot).await?;
            serde_json::json!({ "id": id })
        }
        "CancelScheduledOperation" => {
            let p: CancelScheduledParams = parse_params(params)?;
            manager.cancel_scheduled_operation(p.id).await?;
            serde_json::json!({})
        }

        "GetThermalState" => serde_json::to_value(manager.get_thermal_state().await)?,
        "GetThermalZones" => serde_json::to_value(manager.get_thermal_zones().await)?,

        "AcquireInhibitor" => {
            let p: AcquireInhibitorParams = parse_params(params)?;
            let id = manager.acquire_inhibitor(client_id, p.who, p.why, p.what, p.mode).await;
            serde_json::json!({ "id": id })
        }
        "ReleaseInhibitor" => {
            let p: ReleaseInhibitorParams = parse_params(params)?;
            manager.release_inhibitor(p.id).await?;
            serde_json::json!({})
        }
        "ListInhibitors" => serde_json::to_value(manager.list_inhibitors().await)?,

        "GetLidState" => serde_json::json!({ "closed": manager.get_lid_state().await }),
        "ReportActivity" => { manager.report_activity().await; serde_json::json!({}) }
        "GetIdleState" => serde_json::to_value(manager.get_idle_state().await)?,
        "SetIdleTimeout" => {
            let p: SetIdleTimeoutParams = parse_params(params)?;
            manager.set_idle_timeout(p.secs).await?;
            serde_json::json!({})
        }

        other => return Err(PowerError::UnknownMethod(other.to_string())),
    })
}

fn error_code(e: &PowerError) -> &'static str {
    match e {
        PowerError::Io(_) => "IO_ERROR",
        PowerError::Config(_) => "CONFIG_ERROR",
        PowerError::Hardware(_) => "HARDWARE_ERROR",
        PowerError::NotFound(_) => "NOT_FOUND",
        PowerError::PermissionDenied => "PERMISSION_DENIED",
        PowerError::Inhibited(_) => "INHIBITED",
        PowerError::InvalidState(_) => "INVALID_STATE",
        PowerError::Unsupported(_) => "UNSUPPORTED",
        PowerError::Serde(_) => "INVALID_REQUEST",
        PowerError::Toml(_) => "CONFIG_ERROR",
        PowerError::InvalidParams(_) => "INVALID_PARAMS",
        PowerError::UnknownMethod(_) => "UNKNOWN_METHOD",
        PowerError::Internal(_) => "INTERNAL_ERROR",
    }
}

// Re-exported so `main.rs`/`daemon.rs` can build the ok/error helpers
// without reaching into `protocol` directly if they ever need to.
pub use protocol::{error_response as build_error_response, ok_response as build_ok_response};
