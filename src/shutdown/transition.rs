//! How mitos-power performs the final poweroff/reboot transition.
//!
//! **Not part of the original spec's file tree.** Added when mitos-session
//! was changed to delegate power transitions to mitos-power instead of
//! carrying its own kernel-facing backend: mitos-session used to offer
//! two ways to finish a shutdown (`DirectBackend` and `SupervisedBackend`),
//! and delegating without carrying both over here would have silently
//! removed the supervised one -- the one a full install with
//! mitos-services needs so services stop in dependency order first.
//!
//! - **Direct**: `sync()` then `reboot(2)` ourselves. Correct for a
//!   minimal boot with no service supervisor. This is the default,
//!   for the same reason mitos-session defaulted to it: with nothing to
//!   route through, it's the only choice that works out of the box.
//! - **Supervised**: signal PID 1 (mitos-init), the same way the classic
//!   `reboot`/`poweroff` tools do. mitos-init relays that to
//!   mitos-services, which stops every supervised service in dependency
//!   order and acknowledges; mitos-init then performs the actual
//!   transition. mitos-power never talks to mitos-services directly.
//!   Unlike Direct, this returns as soon as the request is handed off --
//!   the machine goes down moments later.
//!
//! The signal mapping (SIGTERM = power off, SIGINT = reboot) is taken from
//! mitos-services' own `signals.rs` as quoted in mitos-session's
//! `SupervisedBackend`. It has **not** been checked against mitos-init
//! itself -- no mitos-init source was available -- so if that relay's
//! mapping changes, this does too. See audit.md.
//!
//! Suspend and hibernate are unaffected by this setting: they never stop a
//! service (everything stays resident, just frozen), so there is no
//! supervisor sequence to run -- see `sleep::*`.
//!
//! `shutdown::emergency` deliberately does *not* go through this: an
//! emergency thermal poweroff must never wait on a supervisor.

use crate::errors::{PowerError, Result};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransitionBackend {
    #[default]
    Direct,
    Supervised,
}

impl TransitionBackend {
    /// Blocking: callers run this inside `spawn_blocking`.
    pub fn poweroff(self) -> Result<()> {
        match self {
            TransitionBackend::Direct => {
                nix::unistd::sync();
                // On success this never returns -- the machine powers off.
                nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_POWER_OFF)
                    .map_err(|e| PowerError::Hardware(format!("poweroff syscall failed: {e}")))?;
                Ok(())
            }
            TransitionBackend::Supervised => signal_init(Signal::SIGTERM, "poweroff"),
        }
    }

    /// Blocking: callers run this inside `spawn_blocking`.
    pub fn reboot(self) -> Result<()> {
        match self {
            TransitionBackend::Direct => {
                nix::unistd::sync();
                nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_AUTOBOOT)
                    .map_err(|e| PowerError::Hardware(format!("reboot syscall failed: {e}")))?;
                Ok(())
            }
            TransitionBackend::Supervised => signal_init(Signal::SIGINT, "reboot"),
        }
    }
}

fn signal_init(sig: Signal, what: &str) -> Result<()> {
    signal::kill(Pid::from_raw(1), sig).map_err(|e| PowerError::Hardware(format!("could not ask PID 1 to {what}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_from_the_lowercase_names_used_in_power_toml() {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            backend: TransitionBackend,
        }
        let direct: Wrapper = toml::from_str(r#"backend = "direct""#).unwrap();
        let supervised: Wrapper = toml::from_str(r#"backend = "supervised""#).unwrap();
        assert_eq!(direct.backend, TransitionBackend::Direct);
        assert_eq!(supervised.backend, TransitionBackend::Supervised);
        assert!(toml::from_str::<Wrapper>(r#"backend = "bogus""#).is_err());
    }

    #[test]
    fn default_is_direct() {
        assert_eq!(TransitionBackend::default(), TransitionBackend::Direct);
    }
}
