//! `mitos-powerctl`: command-line client for mitos-power, talking over the
//! same Unix-socket IPC protocol documented in docs/ipc.md. Thin by
//! design -- all protocol types come from the `mitos_power` library crate
//! so this binary can never drift from what the daemon actually speaks.

use clap::{Parser, Subcommand};
use mitos_power::ipc::IpcClient;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mitos-powerctl", version, about = "Control the mitos-power daemon")]
struct Cli {
    /// Path to the mitos-power IPC socket.
    #[arg(long, default_value = "/run/mitos/power.sock")]
    socket: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// One-screen summary of current power state
    Status,
    /// Battery details (charge, health, time remaining)
    Battery,
    /// Get or set the active power profile
    Profile {
        /// New profile name (performance, balanced, powersave, or a custom
        /// name from profiles.toml). Omit to print the current profile and
        /// list what's available.
        name: Option<String>,
    },
    /// Get or set screen brightness
    Brightness {
        /// New brightness, 0-100. Omit to print the current value.
        percent: Option<u8>,
    },
    /// Suspend to RAM
    Suspend,
    /// Suspend to disk
    Hibernate,
    /// Suspend to both RAM and disk
    HybridSleep,
    /// Suspend now; hibernate later if nothing wakes it
    SuspendThenHibernate,
    /// Reboot the machine
    Reboot {
        /// Skip the Shutdown-inhibitor check
        #[arg(long)]
        force: bool,
    },
    /// Shut the machine down
    Shutdown {
        /// Skip the Shutdown-inhibitor check
        #[arg(long)]
        force: bool,
    },
    /// Power off immediately (equivalent to Shutdown --force)
    Poweroff,
    /// List currently held power inhibitors
    Inhibitors,
    /// Thermal zones and current level
    Thermal,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut client = IpcClient::connect(&cli.socket)
        .await
        .map_err(|e| anyhow::anyhow!("could not connect to mitos-power at {}: {e}\n\nis the daemon running?", cli.socket.display()))?;

    match cli.command {
        Command::Status => cmd_status(&mut client).await,
        Command::Battery => cmd_battery(&mut client).await,
        Command::Profile { name } => cmd_profile(&mut client, name).await,
        Command::Brightness { percent } => cmd_brightness(&mut client, percent).await,
        Command::Suspend => simple_call(&mut client, "Suspend", serde_json::json!({}), "Suspending...").await,
        Command::Hibernate => simple_call(&mut client, "Hibernate", serde_json::json!({}), "Hibernating...").await,
        Command::HybridSleep => simple_call(&mut client, "HybridSleep", serde_json::json!({}), "Hybrid-sleeping...").await,
        Command::SuspendThenHibernate => {
            simple_call(&mut client, "SuspendThenHibernate", serde_json::json!({}), "Suspending (will hibernate later if untouched)...").await
        }
        Command::Reboot { force } => simple_call(&mut client, "Reboot", serde_json::json!({ "force": force }), "Rebooting...").await,
        Command::Shutdown { force } => simple_call(&mut client, "Shutdown", serde_json::json!({ "force": force }), "Shutting down...").await,
        Command::Poweroff => simple_call(&mut client, "Poweroff", serde_json::json!({}), "Powering off...").await,
        Command::Inhibitors => cmd_inhibitors(&mut client).await,
        Command::Thermal => cmd_thermal(&mut client).await,
    }
}

async fn simple_call(client: &mut IpcClient, method: &str, params: serde_json::Value, message: &str) -> anyhow::Result<()> {
    client.call(method, params).await?;
    println!("{message}");
    Ok(())
}

async fn cmd_status(client: &mut IpcClient) -> anyhow::Result<()> {
    let state = client.call("GetPowerState", serde_json::json!({})).await?;
    let profile = client.call("GetProfile", serde_json::json!({})).await?;

    let on_ac = state["on_ac"].as_bool().unwrap_or(false);
    println!("Power source : {}", if on_ac { "AC" } else { "Battery" });

    if let Some(pct) = state["overall_percentage"].as_f64() {
        println!("Battery      : {pct:.0}%");
    }
    println!("Profile      : {}", profile["name"].as_str().unwrap_or("unknown"));
    println!("Thermal      : {}", state["thermal_level"].as_str().unwrap_or("unknown"));
    if let Some(brightness) = state["display_brightness_percent"].as_u64() {
        println!("Brightness   : {brightness}%");
    }
    println!("Inhibitors   : {} active", state["active_inhibitors"].as_u64().unwrap_or(0));
    println!("Idle for     : {}s", state["idle_seconds"].as_u64().unwrap_or(0));
    Ok(())
}

async fn cmd_battery(client: &mut IpcClient) -> anyhow::Result<()> {
    let batteries = client.call("GetBatteries", serde_json::json!({})).await?;
    let Some(batteries) = batteries.as_array() else {
        println!("No battery data available");
        return Ok(());
    };
    if batteries.is_empty() {
        println!("No battery present (desktop or VM?)");
        return Ok(());
    }

    for b in batteries {
        println!("{}", b["id"].as_str().unwrap_or("?"));
        if let Some(pct) = b["percentage"].as_f64() {
            println!("  Charge   : {pct:.0}%");
        }
        println!("  Status   : {}", b["status"].as_str().unwrap_or("unknown"));
        println!("  Health   : {}", b["health_label"].as_str().unwrap_or("unknown"));
        if let Some(secs) = b["time_to_empty_secs"].as_u64() {
            println!("  Empty in : {}", format_hm(secs));
        }
        if let Some(secs) = b["time_to_full_secs"].as_u64() {
            println!("  Full in  : {}", format_hm(secs));
        }
        if let Some(cycles) = b["cycle_count"].as_u64() {
            println!("  Cycles   : {cycles}");
        }
    }
    Ok(())
}

async fn cmd_profile(client: &mut IpcClient, name: Option<String>) -> anyhow::Result<()> {
    if let Some(name) = name {
        client.call("SetProfile", serde_json::json!({ "name": name })).await?;
        println!("Profile set to '{name}'");
        return Ok(());
    }

    let current = client.call("GetProfile", serde_json::json!({})).await?;
    let current_name = current["name"].as_str().unwrap_or("unknown").to_string();
    println!("Current profile: {current_name}");

    let list = client.call("ListProfiles", serde_json::json!({})).await?;
    if let Some(profiles) = list.as_array() {
        println!("\nAvailable profiles:");
        for p in profiles {
            let name = p["name"].as_str().unwrap_or("?");
            let desc = p["settings"]["description"].as_str().unwrap_or("");
            let marker = if name == current_name { "*" } else { " " };
            println!(" {marker} {name:<14} {desc}");
        }
    }
    Ok(())
}

async fn cmd_brightness(client: &mut IpcClient, percent: Option<u8>) -> anyhow::Result<()> {
    if let Some(percent) = percent {
        client.call("SetBrightness", serde_json::json!({ "percent": percent })).await?;
        println!("Brightness set to {percent}%");
        return Ok(());
    }
    let result = client.call("GetBrightness", serde_json::json!({})).await?;
    println!("Brightness: {}%", result["percent"].as_u64().unwrap_or(0));
    Ok(())
}

async fn cmd_inhibitors(client: &mut IpcClient) -> anyhow::Result<()> {
    let list = client.call("ListInhibitors", serde_json::json!({})).await?;
    let Some(list) = list.as_array() else {
        return Ok(());
    };
    if list.is_empty() {
        println!("No active inhibitors");
        return Ok(());
    }
    for i in list {
        let who = i["who"].as_str().unwrap_or("?");
        let why = i["why"].as_str().unwrap_or("?");
        let what = i["what"].as_str().unwrap_or("?");
        println!("{who:<20} [{what:<8}] {why}");
    }
    Ok(())
}

async fn cmd_thermal(client: &mut IpcClient) -> anyhow::Result<()> {
    let summary = client.call("GetThermalState", serde_json::json!({})).await?;
    println!("Thermal level : {}", summary["level"].as_str().unwrap_or("unknown"));
    if let Some(t) = summary["highest_temp_c"].as_f64() {
        println!("Highest zone  : {t:.1}\u{b0}C");
    }
    println!(
        "Warning / crit: {:.1}\u{b0}C / {:.1}\u{b0}C",
        summary["warning_temp_c"].as_f64().unwrap_or(0.0),
        summary["critical_temp_c"].as_f64().unwrap_or(0.0)
    );

    let zones = client.call("GetThermalZones", serde_json::json!({})).await?;
    if let Some(zones) = zones.as_array() {
        if !zones.is_empty() {
            println!("\nZones:");
            for z in zones {
                let id = z["id"].as_str().unwrap_or("?");
                let zone_type = z["zone_type"].as_str().unwrap_or("?");
                match z["temperature_c"].as_f64() {
                    Some(t) => println!("  {id:<16} {zone_type:<14} {t:.1}\u{b0}C"),
                    None => println!("  {id:<16} {zone_type:<14} (unavailable)"),
                }
            }
        }
    }
    Ok(())
}

fn format_hm(total_secs: u64) -> String {
    format!("{}h {}m", total_secs / 3600, (total_secs % 3600) / 60)
}
