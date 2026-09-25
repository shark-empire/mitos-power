//! Event catalog. Names here are the exact strings used in `Subscribe`
//! requests and `ServerMessage::Event.event`. See docs/ipc.md "Events".

use serde::Serialize;

pub const BATTERY_CHANGED: &str = "BatteryChanged";
pub const CHARGING_CHANGED: &str = "ChargingChanged";
pub const AC_CHANGED: &str = "ACChanged";
pub const THERMAL_WARNING: &str = "ThermalWarning";
pub const THERMAL_CRITICAL: &str = "ThermalCritical";
pub const LID_CHANGED: &str = "LidChanged";
pub const POWER_BUTTON_PRESSED: &str = "PowerButtonPressed";
pub const SUSPEND_STARTED: &str = "SuspendStarted";
pub const SUSPEND_FINISHED: &str = "SuspendFinished";
pub const RESUME_STARTED: &str = "ResumeStarted";
pub const RESUME_FINISHED: &str = "ResumeFinished";
pub const PROFILE_CHANGED: &str = "ProfileChanged";
pub const BRIGHTNESS_CHANGED: &str = "BrightnessChanged";
pub const INHIBITOR_ACQUIRED: &str = "InhibitorAcquired";
pub const INHIBITOR_RELEASED: &str = "InhibitorReleased";
pub const IDLE_STATE_CHANGED: &str = "IdleStateChanged";
pub const SHUTDOWN_SCHEDULED: &str = "ShutdownScheduled";
pub const SHUTDOWN_CANCELLED: &str = "ShutdownCancelled";

/// All valid event names, for validating `Subscribe` requests and for
/// `mitos-powerctl` help text / docs generation.
pub const ALL: &[&str] = &[
    BATTERY_CHANGED,
    CHARGING_CHANGED,
    AC_CHANGED,
    THERMAL_WARNING,
    THERMAL_CRITICAL,
    LID_CHANGED,
    POWER_BUTTON_PRESSED,
    SUSPEND_STARTED,
    SUSPEND_FINISHED,
    RESUME_STARTED,
    RESUME_FINISHED,
    PROFILE_CHANGED,
    BRIGHTNESS_CHANGED,
    INHIBITOR_ACQUIRED,
    INHIBITOR_RELEASED,
    IDLE_STATE_CHANGED,
    SHUTDOWN_SCHEDULED,
    SHUTDOWN_CANCELLED,
];

/// An internal event, as produced by managers and broadcast to the IPC
/// server. `name` is one of the constants above.
#[derive(Debug, Clone)]
pub struct DaemonEvent {
    pub name: &'static str,
    pub data: serde_json::Value,
}

impl DaemonEvent {
    pub fn new(name: &'static str, data: impl Serialize) -> Self {
        Self { name, data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null) }
    }
}
