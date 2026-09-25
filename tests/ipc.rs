//! Live end-to-end IPC test: a real `IpcServer` bound to a real Unix
//! socket (in a temp directory) and a real `IpcClient` talking to it. This
//! is the level of confidence that actually matters for "will mitos-gui/
//! mitos-session's IPC client work" -- protocol-level encode/decode is
//! separately unit-tested in src/ipc/protocol.rs.

use mitos_power::config::Config;
use mitos_power::ipc::{IpcClient, IpcServer};
use mitos_power::manager::PowerManager;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn ping_round_trips_over_a_real_socket() {
    let dir = std::env::temp_dir().join(format!("mitos-power-ipc-test-{}", std::process::id()));
    let socket_path = dir.join("power.sock");

    let manager = Arc::new(PowerManager::new(Config::default()).expect("PowerManager::new should succeed with default config"));
    let server = IpcServer::new(socket_path.clone(), "power".into(), manager);

    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    // Give the listener a moment to bind before connecting.
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut client = IpcClient::connect(&socket_path).await.expect("client should connect to the freshly bound socket");
    let result = client.call("Ping", serde_json::json!({})).await.expect("Ping should succeed");
    assert_eq!(result["pong"], true);

    server_handle.abort();
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn unknown_method_returns_an_error_not_a_panic() {
    let dir = std::env::temp_dir().join(format!("mitos-power-ipc-test-unknown-{}", std::process::id()));
    let socket_path = dir.join("power.sock");

    let manager = Arc::new(PowerManager::new(Config::default()).expect("PowerManager::new should succeed with default config"));
    let server = IpcServer::new(socket_path.clone(), "power".into(), manager);
    let server_handle = tokio::spawn(async move {
        let _ = server.run().await;
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut client = IpcClient::connect(&socket_path).await.expect("client should connect");
    let result = client.call("ThisMethodDoesNotExist", serde_json::json!({})).await;
    assert!(result.is_err());

    server_handle.abort();
    let _ = std::fs::remove_dir_all(&dir);
}
