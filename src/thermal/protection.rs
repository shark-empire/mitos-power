//! Emergency response to Critical thermal state -- a second, faster safety
//! net on top of (never instead of) the kernel's own thermal protection.

pub fn respond_to_critical() {
    tracing::error!("thermal: CRITICAL temperature reached, initiating emergency shutdown");
    crate::shutdown::emergency::emergency_poweroff("thermal critical threshold exceeded");
}
