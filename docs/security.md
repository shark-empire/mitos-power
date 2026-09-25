# Security model

## Trust boundary

The Unix domain socket itself (`/run/mitos/power.sock`, mode `0660`, group
`power` by default) is the primary boundary: only root and members of the
configured group can open a connection at all. Once connected, every method
call is additionally checked against the peer's Unix credentials
(`SO_PEERCRED`, exposed by tokio as `UCred`) -- see `docs/ipc.md`
"Permissions" for the three tiers and the full per-method table.

There is no bearer token, API key, or TLS: this is a local IPC mechanism,
not a network service, and inherits the Unix socket's normal guarantees
(only local processes with filesystem access to the socket path can even
attempt a connection).

## Privileged group membership

`security.privileged_group` (default `power`) is resolved via NSS
(`nix::unistd::Group::from_name` + `User::from_uid`) on every privileged
call, so adding/removing a user from the group takes effect without
restarting the daemon. Lookup failure is treated as "not a member", never as
"allow" -- a broken NSS setup fails closed.

## Running as root

mitos-power needs root today: suspend/hibernate write to `/sys/power/*`,
shutdown/reboot are raw `reboot(2)` syscalls (`CAP_SYS_BOOT`), and CPU
governor/brightness/thermal all write to root-owned sysfs paths. Dropping to
a narrower capability set (`CAP_SYS_BOOT` plus write access to the specific
sysfs subtrees it needs, nothing else) is a known hardening gap -- see
audit.md. The systemd unit in `services/` runs it as root with no further
sandboxing for the same reason; tightening that (ProtectSystem, a capability
allowlist, etc.) is a natural follow-up once the capability-dropping code
itself exists.

## Audit trail

Every privileged action (suspend, hibernate, shutdown, reboot, poweroff,
logout, the emergency-thermal poweroff) is logged through `logging::audit`
at the `audit` tracing target, recording who requested it (`uid:N`, from
peer credentials) and whether it was forced. This currently rides on regular
process logs rather than a dedicated rotated file -- see audit.md.

## What's explicitly out of scope

mitos-power does not authenticate *which application* is calling beyond its
Unix uid/gid -- "any process running as your user can call SetBrightness" is
the intended model (the same trust level X11/Wayland apps already have for
much more sensitive things). If MITOS later wants sandboxed apps to have a
narrower IPC surface than the logged-in user, that needs a capability/policy
layer on top of this, not a change to mitos-power's own socket.
