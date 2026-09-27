//! `PowerManager`: owns every subsystem manager and is the one object both
//! the IPC server (`ipc::server::dispatch_inner`) and the internal policy
//! engine call into. One method here maps to (usually) exactly one IPC
//! method -- see docs/ipc.md for the wire-level view of this same surface.

use crate::ac::AcManager;
use crate::battery::BatteryManager;
use crate::config::{Config, SecurityConfig};
use crate::cpu::CpuController;
use crate::display::{BrightnessController, DisplayPowerController, DisplayPowerState};
use crate::errors::{PowerError, Result};
use crate::idle::{IdleDetector, IdleState};
use crate::inhibitor::{InhibitMode, InhibitWhat, InhibitorInfo, InhibitorManager};
use crate::ipc::events::{self, DaemonEvent};
use crate::manager::policy::{Action, PolicyEngine};
use crate::manager::scheduler::Scheduler;
use crate::manager::state::PowerStateSnapshot;
use crate::monitoring::{diagnostics, statistics::Metrics, Diagnostics, HealthTracker};
use crate::persistence::{self, Database};
use crate::profiles::{ProfileInfo, ProfileManager};
use crate::shutdown;
use crate::sleep;
use crate::thermal::{ThermalLevel, ThermalManager, ThermalSummary, ThermalZoneInfo};
use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

pub struct PowerManager {
    pub config: RwLock<Config>,
    pub battery: RwLock<BatteryManager>,
    pub ac: RwLock<AcManager>,
    pub profiles: RwLock<ProfileManager>,
    pub thermal: RwLock<ThermalManager>,
    pub inhibitors: InhibitorManager,
    pub brightness: BrightnessController,
    pub display_power: RwLock<DisplayPowerController>,
    pub cpu: CpuController,
    pub idle: RwLock<IdleDetector>,
    pub scheduler: Scheduler,
    pub lid_closed: RwLock<Option<bool>>,
    pub events_tx: broadcast::Sender<DaemonEvent>,
    /// File-backed settings/statistics/history storage. See
    /// `restore_persisted_state` for what actually gets loaded/saved
    /// through it today.
    pub db: Database,
    /// Runtime counters (IPC requests served, suspend/resume counts,
    /// policy actions executed). Exposed read-only via `GetDiagnostics`.
    pub metrics: Metrics,
    health: HealthTracker,
    policy_engine: PolicyEngine,
}

impl PowerManager {
    pub fn new(config: Config) -> Result<Self> {
        let (events_tx, _rx) = broadcast::channel(256);
        let profiles = ProfileManager::new(&config.profiles.default_profile, config.profiles.profile.clone())?;
        let idle = IdleDetector::new(&config.display);
        let thermal = ThermalManager::new(&config.thermal)?;
        let db = Database::open(&config.general.state_dir)?;

        Ok(Self {
            battery: RwLock::new(BatteryManager::new()?),
            ac: RwLock::new(AcManager::new()?),
            profiles: RwLock::new(profiles),
            thermal: RwLock::new(thermal),
            inhibitors: InhibitorManager::default(),
            brightness: BrightnessController::new()?,
            display_power: RwLock::new(DisplayPowerController::new()),
            cpu: CpuController::new(),
            idle: RwLock::new(idle),
            scheduler: Scheduler::default(),
            lid_closed: RwLock::new(None),
            config: RwLock::new(config),
            events_tx,
            db,
            metrics: Metrics::default(),
            health: HealthTracker::new(),
            policy_engine: PolicyEngine::new(),
        })
    }

    /// Loads whatever was last persisted (`persistence::settings`) and
    /// applies it -- last active profile, last brightness. Called once by
    /// `Daemon::run` before the IPC server starts accepting connections.
    /// Best-effort: a restore failure is logged, not propagated, since a
    /// corrupt/stale state file shouldn't stop the daemon from starting
    /// with built-in defaults instead.
    pub async fn restore_persisted_state(&self) {
        let settings = match persistence::settings::load(&self.db) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("could not load persisted settings, starting with defaults: {e}");
                return;
            }
        };

        if let Some(name) = &settings.last_profile {
            if let Err(e) = self.set_profile(name).await {
                tracing::warn!("could not restore persisted profile '{name}': {e}");
            } else {
                tracing::info!("restored profile '{name}' from last run");
            }
        }
        if let Some(percent) = settings.last_brightness_percent {
            if let Err(e) = self.set_brightness(percent).await {
                tracing::warn!("could not restore persisted brightness {percent}%: {e}");
            } else {
                tracing::info!("restored brightness {percent}% from last run");
            }
        }
    }

    /// Best-effort persist of one field. Reads the current persisted
    /// settings, updates just the field the caller changed, and saves --
    /// a read-modify-write, so two concurrent callers (e.g. SetProfile and
    /// SetBrightness from two different clients at nearly the same moment)
    /// could in principle race and one write could clobber the other's
    /// update. Given this is a "restore roughly where you left off" best
    /// effort, not a strongly-consistent store, that narrow window is an
    /// accepted tradeoff rather than something worth a write-queue for.
    ///
    /// Also deliberately synchronous rather than `spawn_blocking`'d: this
    /// briefly blocks whatever tokio worker thread called it on a few KB
    /// of local disk I/O. Acceptable for how small/rare this write is;
    /// worth revisiting with spawn_blocking if `persist_setting` ever
    /// grows to write more than a tiny settings file.
    fn persist_setting(&self, apply: impl FnOnce(&mut persistence::settings::PersistedSettings)) {
        let mut settings = persistence::settings::load(&self.db).unwrap_or_default();
        apply(&mut settings);
        if let Err(e) = persistence::settings::save(&self.db, &settings) {
            tracing::warn!("could not persist settings: {e}");
        }
    }

    fn emit(&self, name: &'static str, data: impl serde::Serialize) {
        // A send error just means no client is currently subscribed --
        // that's the normal case, not a failure.
        let _ = self.events_tx.send(DaemonEvent::new(name, data));
    }

    // ---- state -------------------------------------------------------

    pub async fn get_power_state(&self) -> PowerStateSnapshot {
        let battery = self.battery.read().await;
        let ac = self.ac.read().await;
        let profile = self.profiles.read().await;
        let thermal = self.thermal.read().await;
        PowerStateSnapshot {
            on_ac: ac.is_online(),
            batteries: battery.all(),
            overall_percentage: battery.overall_percentage(),
            profile: profile.current().kind.clone(),
            lid_closed: *self.lid_closed.read().await,
            display_brightness_percent: self.brightness.get_percent().ok(),
            thermal_level: thermal.level(),
            active_inhibitors: self.inhibitors.list().await.len(),
            idle_seconds: self.idle.read().await.idle_seconds(),
        }
    }

    /// Re-reads battery/AC/thermal and emits change events, without running
    /// the policy engine. Safe to call from `sleep::wake::on_resume` --
    /// unlike `refresh_and_run_policy`, this can never itself trigger a new
    /// Suspend/Hibernate action, which matters because a resume handler
    /// that re-ran the full policy tick could re-trigger the very action
    /// that just woke up (e.g. battery still critical immediately after
    /// waking from a policy-triggered hibernate) and recurse without bound.
    pub async fn refresh_hardware_state(&self) {
        self.refresh_battery().await;
        self.refresh_ac().await;
        self.refresh_thermal().await;
    }

    /// Evaluates the policy engine against the current state and executes
    /// whatever actions it returns. Called after `refresh_hardware_state`
    /// on the daemon's regular poll tick -- deliberately *not* called from
    /// the resume path (see `refresh_hardware_state` doc comment).
    pub async fn run_policy_tick(&self) {
        let state = self.get_power_state().await;
        let actions = {
            let config = self.config.read().await;
            self.policy_engine.evaluate(&state, &config)
        };
        for action in actions {
            self.execute_action(action).await;
        }
    }

    /// Re-read every hardware subsystem and emit the appropriate events for
    /// whatever changed. Called on a timer by `daemon::event_loop` and
    /// after every uevent, then immediately followed by a policy tick.
    pub async fn refresh_and_run_policy(&self) {
        self.refresh_hardware_state().await;
        self.run_policy_tick().await;
    }

    async fn refresh_battery(&self) {
        let result = { self.battery.write().await.refresh() };
        match result {
            Ok(r) => {
                if r.percentage_changed {
                    self.emit(events::BATTERY_CHANGED, self.battery.read().await.all());
                }
                if r.status_changed {
                    self.emit(events::CHARGING_CHANGED, self.battery.read().await.all());
                }
            }
            Err(e) => tracing::warn!("battery refresh failed: {e}"),
        }
    }

    async fn refresh_ac(&self) {
        let changed = { self.ac.write().await.refresh() };
        match changed {
            Ok(true) => self.emit(events::AC_CHANGED, serde_json::json!({ "online": self.ac.read().await.is_online() })),
            Ok(false) => {}
            Err(e) => tracing::warn!("AC refresh failed: {e}"),
        }
    }

    async fn refresh_thermal(&self) {
        let previous = self.thermal.read().await.level();
        let result = { self.thermal.write().await.refresh() };
        match result {
            Ok(new_level) => {
                if new_level != previous {
                    match new_level {
                        ThermalLevel::Warning => self.emit(events::THERMAL_WARNING, self.thermal.read().await.summary()),
                        ThermalLevel::Critical => {
                            self.emit(events::THERMAL_CRITICAL, self.thermal.read().await.summary());
                            crate::thermal::protection::respond_to_critical();
                        }
                        ThermalLevel::Nominal => {}
                    }
                }
            }
            Err(e) => tracing::warn!("thermal refresh failed: {e}"),
        }
    }

    async fn execute_action(&self, action: Action) {
        self.metrics.incr_policy_actions();
        match action {
            Action::SetProfile(name) => {
                if let Err(e) = self.set_profile(&name).await {
                    tracing::warn!("policy: SetProfile({name}) failed: {e}");
                }
            }
            Action::Suspend => {
                if let Err(e) = self.suspend("policy-engine").await {
                    tracing::warn!("policy: Suspend failed: {e}");
                }
            }
            Action::Hibernate => {
                if let Err(e) = self.hibernate("policy-engine").await {
                    tracing::warn!("policy: Hibernate failed: {e}");
                }
            }
            Action::Shutdown => {
                // Non-forced: a held Shutdown inhibitor ("backup running")
                // is still respected here. If the battery is truly about to
                // die, the firmware/kernel's own critical-battery poweroff
                // is the actual last resort -- mitos-power can't prevent
                // that regardless of what this policy does.
                if let Err(e) = self.shutdown("policy-engine", false).await {
                    tracing::warn!("policy: Shutdown failed or was inhibited: {e}");
                }
            }
            Action::Notify { title, body } => {
                tracing::info!(target: "mitos_power::policy_notify", "{title}: {body}");
            }
        }
    }

    // ---- battery / ac --------------------------------------------------

    pub async fn get_battery(&self, id: Option<&str>) -> Result<crate::battery::BatteryInfo> {
        let battery = self.battery.read().await;
        match id {
            Some(id) => battery.get(id).ok_or_else(|| PowerError::NotFound(format!("battery '{id}'"))),
            None => battery.all().into_iter().next().ok_or_else(|| PowerError::NotFound("no battery present".into())),
        }
    }

    pub async fn get_batteries(&self) -> Vec<crate::battery::BatteryInfo> {
        self.battery.read().await.all()
    }

    pub async fn get_ac_state(&self) -> crate::ac::AcState {
        self.ac.read().await.state()
    }

    // ---- profiles --------------------------------------------------------

    pub async fn get_profile(&self) -> ProfileInfo {
        self.profiles.read().await.current().clone()
    }

    pub async fn list_profiles(&self) -> Vec<ProfileInfo> {
        self.profiles.read().await.list().to_vec()
    }

    pub async fn set_profile(&self, name: &str) -> Result<()> {
        let info = {
            let mut profiles = self.profiles.write().await;
            profiles.set(name)?.clone()
        };
        self.cpu.apply_governor(&info.settings.cpu_governor)?;
        self.cpu.set_boost(info.settings.cpu_boost)?;

        let display_config = self.config.read().await.display.clone();
        let timeouts = crate::idle::policy::effective_timeouts(&display_config, &info);
        self.idle.write().await.set_timeouts(timeouts);

        self.persist_setting(|s| s.last_profile = Some(info.name.clone()));
        self.emit(events::PROFILE_CHANGED, &info);
        Ok(())
    }

    // ---- display -----------------------------------------------------

    pub async fn get_brightness(&self) -> Result<u8> {
        self.brightness.get_percent()
    }

    pub async fn set_brightness(&self, percent: u8) -> Result<()> {
        let percent = percent.min(100);
        self.brightness.set_percent(percent)?;
        self.persist_setting(|s| s.last_brightness_percent = Some(percent));
        self.emit(events::BRIGHTNESS_CHANGED, serde_json::json!({ "percent": percent }));
        Ok(())
    }

    // ---- sleep -------------------------------------------------------

    async fn ensure_not_inhibited(&self, what: InhibitWhat) -> Result<()> {
        let (blocking, delaying) = self.inhibitors.split_blockers(what).await;
        if !blocking.is_empty() {
            return Err(PowerError::Inhibited(blocking));
        }
        if !delaying.is_empty() {
            let grace = std::time::Duration::from_secs(self.config.read().await.general.inhibitor_delay_grace_secs);
            tracing::info!("delaying {what:?} up to {grace:?} for: {delaying:?}");
            if !self.inhibitors.wait_for_delay_clear(what, grace).await {
                tracing::info!("delay grace period elapsed for {what:?}; proceeding anyway");
            }
        }
        Ok(())
    }

    async fn around_sleep<F, Fut>(&self, requester: &str, method: &'static str, op: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        self.ensure_not_inhibited(InhibitWhat::Suspend).await?;
        self.emit(events::SUSPEND_STARTED, serde_json::json!({ "requester": requester, "method": method }));
        self.metrics.incr_suspend();
        op().await?;
        self.emit(events::SUSPEND_FINISHED, serde_json::json!({ "method": method }));
        self.emit(events::RESUME_STARTED, serde_json::json!({}));
        self.metrics.incr_resume();
        sleep::wake::on_resume(self).await?;
        self.emit(events::RESUME_FINISHED, serde_json::json!({}));
        Ok(())
    }

    pub async fn suspend(&self, requester: &str) -> Result<()> {
        self.around_sleep(requester, "suspend", sleep::suspend::suspend).await
    }

    pub async fn hibernate(&self, requester: &str) -> Result<()> {
        self.around_sleep(requester, "hibernate", sleep::hibernate::hibernate).await
    }

    pub async fn hybrid_sleep(&self, requester: &str) -> Result<()> {
        self.around_sleep(requester, "hybrid_sleep", sleep::hybrid_sleep::hybrid_sleep).await
    }

    pub async fn suspend_then_hibernate(&self, requester: &str) -> Result<()> {
        let delay = self.config.read().await.sleep.suspend_then_hibernate_delay_mins;
        self.around_sleep(requester, "suspend_then_hibernate", move || {
            sleep::suspend_then_hibernate::suspend_then_hibernate(delay)
        })
        .await
    }

    // ---- shutdown / reboot ---------------------------------------------

    pub async fn shutdown(&self, requester: &str, force: bool) -> Result<()> {
        if !force {
            self.ensure_not_inhibited(InhibitWhat::Shutdown).await?;
        }
        shutdown::shutdown::shutdown(requester, force).await
    }

    pub async fn reboot(&self, requester: &str, force: bool) -> Result<()> {
        if !force {
            self.ensure_not_inhibited(InhibitWhat::Shutdown).await?;
        }
        shutdown::reboot::reboot(requester, force).await
    }

    pub async fn poweroff(&self, requester: &str) -> Result<()> {
        shutdown::poweroff::poweroff(requester).await
    }

    pub async fn logout(&self, requester: &str) -> Result<()> {
        shutdown::logout::logout(requester).await
    }

    pub async fn schedule_shutdown(&self, at: DateTime<Utc>, reboot: bool) -> Result<Uuid> {
        let id = self.scheduler.schedule(at, reboot);
        self.emit(events::SHUTDOWN_SCHEDULED, serde_json::json!({ "id": id, "at": at, "reboot": reboot }));
        Ok(id)
    }

    pub async fn cancel_scheduled_operation(&self, id: Uuid) -> Result<()> {
        self.scheduler.cancel(id)?;
        self.emit(events::SHUTDOWN_CANCELLED, serde_json::json!({ "id": id }));
        Ok(())
    }

    // ---- thermal -------------------------------------------------------

    pub async fn get_thermal_state(&self) -> ThermalSummary {
        self.thermal.read().await.summary()
    }

    pub async fn get_thermal_zones(&self) -> Vec<ThermalZoneInfo> {
        self.thermal.read().await.zones()
    }

    // ---- inhibitors --------------------------------------------------

    pub async fn acquire_inhibitor(&self, client_id: Uuid, who: String, why: String, what: InhibitWhat, mode: InhibitMode) -> Uuid {
        let id = self.inhibitors.acquire(client_id, who.clone(), why.clone(), what, mode).await;
        self.emit(events::INHIBITOR_ACQUIRED, serde_json::json!({ "id": id, "who": who, "why": why, "what": what }));
        id
    }

    pub async fn release_inhibitor(&self, id: Uuid) -> Result<()> {
        self.inhibitors.release(id).await?;
        self.emit(events::INHIBITOR_RELEASED, serde_json::json!({ "id": id }));
        Ok(())
    }

    pub async fn list_inhibitors(&self) -> Vec<InhibitorInfo> {
        self.inhibitors.list().await
    }

    // ---- lid / idle ----------------------------------------------------

    pub async fn get_lid_state(&self) -> Option<bool> {
        *self.lid_closed.read().await
    }

    /// Called by `devices::lid` when a uevent/ACPI lid-switch event arrives.
    pub async fn set_lid_state(&self, closed: bool) {
        let mut guard = self.lid_closed.write().await;
        if *guard == Some(closed) {
            return;
        }
        *guard = Some(closed);
        drop(guard);
        self.emit(events::LID_CHANGED, serde_json::json!({ "closed": closed }));
    }

    pub async fn report_activity(&self) {
        let woke = { self.idle.write().await.reset() };
        if woke {
            self.notify_idle_state_changed(0, crate::display::timeout::DisplayTier::Awake).await;
            self.display_power.write().await.set_state(DisplayPowerState::On);
        }
    }

    pub async fn get_idle_state(&self) -> IdleState {
        self.idle.read().await.state()
    }

    pub async fn set_idle_timeout(&self, secs: u64) -> Result<()> {
        self.idle.write().await.set_off_timeout(secs);
        Ok(())
    }

    /// Called by `daemon::event_loop` when the idle detector's tier changes
    /// (Awake/Dim/Off). Kept as its own method, rather than exposing the
    /// private `emit()` helper, so event construction stays in one place.
    pub async fn notify_idle_state_changed(&self, idle_seconds: u64, tier: crate::display::timeout::DisplayTier) {
        self.emit(events::IDLE_STATE_CHANGED, serde_json::json!({ "idle_seconds": idle_seconds, "tier": tier }));
    }

    /// Called by `devices::power_button::apply`, regardless of whether the
    /// classified action is `Ignore` -- subscribers may want to know a
    /// press happened even if configured policy did nothing with it.
    pub async fn notify_power_button_pressed(&self, action: crate::devices::power_button::ButtonAction) {
        self.emit(events::POWER_BUTTON_PRESSED, serde_json::json!({ "action": format!("{action:?}") }));
    }

    pub async fn security_config(&self) -> SecurityConfig {
        self.config.read().await.security.clone()
    }

    /// Backs the `GetDiagnostics` IPC method -- uptime, version, detected
    /// hardware capabilities, and the runtime metrics counters. This is
    /// what makes `monitoring::statistics::Metrics` (incremented above in
    /// `around_sleep`/`execute_action`/`ipc::server::dispatch`) actually
    /// observable from outside the process.
    pub async fn get_diagnostics(&self) -> Diagnostics {
        diagnostics::collect(self.health.uptime().as_secs(), self.metrics.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config() -> Config {
        let mut config = Config::default();
        config.general.state_dir =
            std::env::temp_dir().join(format!("mitos-power-manager-test-{}-{}", std::process::id(), uuid::Uuid::new_v4())).to_string_lossy().to_string();
        config
    }

    #[tokio::test]
    async fn restores_persisted_profile_across_a_fresh_instance() {
        // Proves the persistence wiring added to close the audit.md gap
        // actually round-trips, not just that it compiles: two separate
        // `PowerManager` instances sharing the same state_dir, simulating
        // a daemon restart.
        let config = temp_config();
        let state_dir = config.general.state_dir.clone();

        {
            let manager = PowerManager::new(config.clone()).expect("first PowerManager::new should succeed");
            manager.set_profile("performance").await.expect("performance is always available");
        }

        let manager2 = PowerManager::new(config).expect("second PowerManager::new (same state_dir) should succeed");
        manager2.restore_persisted_state().await;
        assert_eq!(manager2.get_profile().await.name, "performance");

        let _ = std::fs::remove_dir_all(&state_dir);
    }

    #[tokio::test]
    async fn diagnostics_reflects_incremented_metrics() {
        let manager = PowerManager::new(temp_config()).expect("PowerManager::new should succeed");
        manager.metrics.incr_ipc_requests();
        manager.metrics.incr_ipc_requests();
        let diagnostics = manager.get_diagnostics().await;
        assert_eq!(diagnostics.metrics.ipc_requests_total, 2);
        let _ = std::fs::remove_dir_all(&manager.config.read().await.general.state_dir);
    }
}
