//! The one path in mitos-power that skips every graceful step. Called only
//! from `thermal::protection` when the system is in a state where waiting
//! for the normal shutdown flow risks hardware damage. Deliberately
//! synchronous (no tokio) so it can be called from contexts that can't
//! `.await`.

use crate::logging::audit;

pub fn emergency_poweroff(reason: &str) {
    tracing::error!("EMERGENCY POWEROFF: {reason}");
    audit::log_privileged_action("mitos-power(emergency)", &format!("emergency_poweroff: {reason}"), true);
    nix::unistd::sync();
    if let Err(e) = nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_POWER_OFF) {
        tracing::error!("emergency poweroff syscall itself failed: {e}");
    }
}
