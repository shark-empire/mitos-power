//! Advisory helper: which sleep method should `Suspend` actually use, given
//! current conditions. Not wired into `PowerManager::suspend()` by default
//! (callers explicitly choose suspend/hibernate/hybrid_sleep/suspend_then_hibernate
//! over IPC), but available for a future "just do the right thing" mode,
//! e.g. a GUI "Sleep" button that wants mitos-power to pick.

use crate::config::SleepConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepMethod {
    Suspend,
    Hibernate,
    HybridSleep,
    SuspendThenHibernate,
}

/// If the battery is critically low, prefer hibernate (survives a fully
/// depleted battery) over whatever the configured default is; otherwise
/// use `sleep.default_method` from sleep.toml.
pub fn choose(config: &SleepConfig, on_ac: bool, battery_percent: Option<f32>, critical_threshold: u8) -> SleepMethod {
    if !on_ac {
        if let Some(pct) = battery_percent {
            if pct <= critical_threshold as f32 {
                return SleepMethod::Hibernate;
            }
        }
    }
    match config.default_method.as_str() {
        "hibernate" => SleepMethod::Hibernate,
        "hybrid_sleep" => SleepMethod::HybridSleep,
        "suspend_then_hibernate" => SleepMethod::SuspendThenHibernate,
        _ => SleepMethod::Suspend,
    }
}
