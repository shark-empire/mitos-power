# Suspend / sleep

mitos-power talks to the kernel's raw `/sys/power/*` interface directly --
no systemd-logind dependency, consistent with MITOS having its own service
manager (mitos-services).

| Method | Kernel interface | Notes |
|---|---|---|
| Suspend | `echo mem > /sys/power/state` (falls back to `freeze`) | RAM stays powered; fastest resume |
| Hibernate | `echo disk > /sys/power/state` | Needs swap sized >= RAM and `resume=` on the kernel cmdline |
| Hybrid sleep | `echo suspend > /sys/power/disk` then `echo disk > /sys/power/state` | Survives a fully depleted battery during sleep |
| Suspend-then-hibernate | RTC wakealarm + suspend; hibernates on alarm, resumes normally if woken early | Mirrors systemd-logind's semantics without depending on it |

All four block the calling OS thread until the machine actually resumes, so
every call happens inside `tokio::task::spawn_blocking` -- never directly on
an async task, or it would stall the whole tokio runtime for as long as the
system is asleep.

## Flow

```
IPC Suspend/Hibernate/...
        |
check Suspend inhibitors  (blocked? -> INHIBITED error, nothing happens)
        |
best-effort: lock every mitos-session session (session_client)
        |
emit SuspendStarted
        |
spawn_blocking: write /sys/power/*   <-- blocks here until hardware wakes
        |
emit SuspendFinished, ResumeStarted
        |
sleep::wake::on_resume: restore CPU governor/boost, reset idle timer,
                         refresh battery/AC/thermal (policy-free -- see
                         docs/architecture.md "recursion hazard")
        |
emit ResumeFinished
```

Locking is mitos-session's responsibility, not mitos-power's -- but
mitos-power asks it to lock every session before every suspend/hibernate
regardless of who triggered it (see docs/architecture.md "mitos-session
integration"), since not every path here goes through mitos-session's own
`Suspend` request (which already locks first on its own). Unlocking after
resume is mitos-session's alone; mitos-power has no part in it.
`SuspendStarted`/`ResumeFinished` are the events any other component should
subscribe to if it needs to react around sleep too.

## Shutdown / reboot / poweroff

Graceful path: audit log -> `force: false` (the default) checks Shutdown
inhibitors, `force: true` skips that check -> best-effort wind down every
mitos-session session (`session_client::terminate_all_sessions`, giving
applications a chance to exit) -> sync filesystems (`nix::unistd::sync`) ->
the configured `shutdown.backend` transition (`shutdown::transition`):
either a raw `reboot(2)` syscall directly (`direct`, the default -- works
with no other MITOS components running), or a signal to PID 1 so
mitos-services stops every supervised service in dependency order first
(`supervised` -- see `src/shutdown/transition.rs`). The one path that
skips everything -- inhibitor check, session wind-down, and the audit
log's usual place in the flow -- is `shutdown::emergency::emergency_poweroff`,
called only by `thermal::protection` on a Critical reading, which always
goes straight to the kernel regardless of `shutdown.backend`.

## Known constraints

- Hibernate/hybrid-sleep need swap >= RAM size and `resume=` configured; this
  is not verified before attempting the write (see `docs/troubleshooting.md`).
- All of this needs `CAP_SYS_BOOT` (in practice: root). See `docs/security.md`.
