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

Locking the session before suspend and unlocking after resume is
`mitos-session`'s job, not mitos-power's -- `SuspendStarted`/`ResumeFinished`
are exactly the events mitos-session should subscribe to for that.

## Shutdown / reboot / poweroff

Graceful path: audit log -> sync filesystems (`nix::unistd::sync`) -> raw
`reboot(2)` syscall (`RB_POWER_OFF` / `RB_AUTOBOOT`). `force: true` skips the
Shutdown-inhibitor check; `force: false` (the default) respects it. The one
path that skips everything, including the inhibitor check and the audit
log's usual place in the flow, is `shutdown::emergency::emergency_poweroff`,
called only by `thermal::protection` on a Critical reading.

## Known constraints

- Hibernate/hybrid-sleep need swap >= RAM size and `resume=` configured; this
  is not verified before attempting the write (see `docs/troubleshooting.md`).
- All of this needs `CAP_SYS_BOOT` (in practice: root). See `docs/security.md`.
