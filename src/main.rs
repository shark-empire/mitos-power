//! mitos-power daemon entry point. Thin by design: everything meaningful
//! lives in the `mitos_power` library crate (src/lib.rs) so it can also be
//! exercised by `mitos-powerctl` and by integration tests.

use clap::Parser;
use mitos_power::config::Config;
use mitos_power::daemon::Daemon;
use std::path::PathBuf;

/// MITOS power-management daemon.
#[derive(Parser, Debug)]
#[command(name = "mitos-power", version, about)]
struct Args {
    /// Directory containing power.toml, profiles.toml, battery.toml,
    /// sleep.toml, display.toml, thermal.toml.
    #[arg(long, default_value = "/etc/mitos/power")]
    config_dir: PathBuf,

    /// Run in the foreground with logs on stderr (the default -- mitos-power
    /// doesn't background/fork itself; use your service manager for that).
    #[arg(long)]
    foreground: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    mitos_power::logging::init();

    tracing::info!("loading config from {}", args.config_dir.display());
    let config = Config::load(&args.config_dir).unwrap_or_else(|e| {
        tracing::warn!("failed to load config ({e}); continuing with built-in defaults");
        Config::default()
    });

    let daemon = Daemon::new(config, args.config_dir)?;
    daemon.run().await?;
    Ok(())
}
