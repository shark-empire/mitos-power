//! Permission checks for IPC methods, based on the connecting client's Unix
//! socket peer credentials (uid/gid) -- there is no separate auth token;
//! the socket itself, plus standard Unix credentials passing, is the trust
//! boundary. See docs/ipc.md "Permissions".

use super::messages::method_names;
use crate::errors::{PowerError, Result};
use tokio::net::unix::UCred;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Privilege {
    /// Any client that can open the socket at all.
    Public,
    /// Any authenticated local Unix user. (Future: restrict to the active
    /// seat via logind-equivalent session tracking.)
    Session,
    /// root, or a member of `security.privileged_group` (default `power`).
    Privileged,
}

pub fn required_privilege(method: &str) -> Privilege {
    if method_names::PUBLIC.contains(&method) {
        Privilege::Public
    } else if method_names::SESSION.contains(&method) {
        Privilege::Session
    } else if method_names::PRIVILEGED.contains(&method) {
        Privilege::Privileged
    } else {
        // Unknown methods default to the strictest tier; the dispatcher
        // will return UNKNOWN_METHOD right after this check passes/fails,
        // but we don't want an unrecognized name to sneak past as "public".
        Privilege::Privileged
    }
}

pub fn check(method: &str, cred: &UCred, privileged_group: &str) -> Result<()> {
    match required_privilege(method) {
        Privilege::Public => Ok(()),
        Privilege::Session => Ok(()),
        Privilege::Privileged => {
            if cred.uid() == 0 {
                return Ok(());
            }
            if user_in_group(cred.uid(), privileged_group) {
                return Ok(());
            }
            Err(PowerError::PermissionDenied)
        }
    }
}

/// Look up whether the user owning `uid` belongs to `group_name` (by primary
/// or supplementary group). Best-effort: treats lookup failure as "not a
/// member" rather than panicking, since a misconfigured NSS setup shouldn't
/// crash the daemon.
fn user_in_group(uid: u32, group_name: &str) -> bool {
    let group = match nix::unistd::Group::from_name(group_name) {
        Ok(Some(g)) => g,
        _ => return false,
    };

    let user = match nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid)) {
        Ok(Some(u)) => u,
        _ => return false,
    };

    if user.gid == group.gid {
        return true;
    }
    group.mem.iter().any(|member| member == &user.name)
}
