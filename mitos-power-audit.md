# Audit: what's real vs. what's not

This was written in a sandbox with **no Rust toolchain and no network
access** (verified: `cargo`/`rustc` aren't installed, and crates.io is
unreachable). Every file here is hand-written, careful, idiomatic Rust
against APIs I'm confident about -- but none of it has been through
`cargo build`, `cargo test`, or even `cargo check`. Treat this as a strong
first draft that needs a real build pass, not verified-working code.

Legend: ✅ implemented and, to the best of my ability without a compiler,
correct · ⚠️ implemented but with a real, documented limitation · ‼️ not
implemented (structure/interface may exist; the behavior doesn't)

## 0. Compilation

‼️ **Never compiled.** No `cargo build` / `cargo check` was run against this
code, at all, for any file. First thing to do on a real machine:
```
cd mitos-power && cargo build
```
Expect some errors -- 140+ files of hand-written Rust with zero compiler
feedback will not be perfect. The two areas I'd bet on first if something
doesn't compile:
- **`nix` crate feature flags** (Cargo.toml) -- I listed `reboot`, `user`,
  `fs`; the exact feature names have moved around across `nix` 0.2x
  releases. `nix::sys::reboot::reboot`/`RebootMode`, `nix::unistd::sync`,
  `nix::unistd::{User, Group}` are the APIs actually used
  (`src/shutdown/*.rs`, `src/ipc/permissions.rs`).
- **`udev` crate API shape** (`src/hardware/uevent.rs`) -- `MonitorBuilder`/
  `match_subsystem`/`listen`/`.iter()` on the returned socket is the API I
  wrote against; double-check method names against whatever `udev` version
  actually resolves.

Every other dependency (`tokio`, `serde`/`serde_json`, `toml`, `thiserror`,
`tracing`, `clap`, `chrono`, `uuid`, `anyhow`) I'm confident about at the API
level used here.

## 1. Core subsystems

✅ **errors** -- complete.
✅ **hardware/sysfs** -- complete, unit tested.
✅ **hardware/acpi, capabilities** -- complete.
⚠️ **hardware/uevent** -- implemented via the `udev` crate on a dedicated
OS thread bridged into tokio via `mpsc`; see the compilation note above.
✅ **battery** (all 11 files) -- complete: discovery, capacity/health/
voltage/current/temperature/time-remaining math, rolling statistics, all
unit tested against synthetic data (no real sysfs needed for the tests).
✅ **ac** -- complete, `type == "Mains"` detection.
✅ **display/backlight, brightness, timeout** -- complete, real sysfs I/O.
⚠️ **display/display_power, night_mode** -- state tracking only, by design:
mitos-power doesn't own the compositor, so actual blanking/color-temp shift
is delegated to mitos-gui via events (see docs/architecture.md). Not a gap,
a deliberate boundary -- flagged here so it's not mistaken for one.
✅ **profiles** -- complete: built-ins, custom-profile TOML loading,
manager, switching.
✅ **inhibitor** -- complete: acquire/release/list/blockers, auto-release on
client disconnect, unit tested.
✅ **cpu** -- complete: governor (Intel+generic paths), boost (Intel +
AMD-pstate paths), frequency/cores/topology introspection.
✅ **thermal** -- complete: zone discovery, trip-point reading, Warning/
Critical classification, emergency-shutdown trigger on Critical.
✅ **idle** -- complete: tier tracking, per-profile timeout recalculation,
Idle-inhibitor gating, unit tested including the "would deadlock the
suspend->resume->suspend policy loop" fix described in
docs/architecture.md.

## 2. Sleep / shutdown

⚠️ **sleep** (suspend/hibernate/hybrid_sleep/suspend_then_hibernate) --
fully implemented against the real `/sys/power/*` kernel interface,
including the suspend-then-hibernate RTC-wakealarm heuristic. **Cannot be
exercised at all in this sandbox** (no `/sys/power/state`) and genuinely
cannot be safety-tested anywhere without a disposable VM -- see
`tests/suspend.rs`/`hibernate.rs`, which `#[ignore]` every test that would
actually call these.
⚠️ **shutdown/reboot/poweroff** -- same situation: real `reboot(2)` syscalls
via `nix`, untestable outside a disposable VM, tests `#[ignore]`d
accordingly.
‼️ **shutdown/logout** -- **explicitly a no-op.** mitos-power doesn't have an
IPC client to mitos-session (which owns sessions), so `Logout` currently
just audit-logs and returns `Ok`. This is the single most important
documented integration gap -- see docs/ipc.md "Who calls what" and
docs/architecture.md "Integration".

## 3. Input devices

✅ **devices/usb, wakeup** -- complete, real sysfs (`power/wakeup`,
`/sys/class/wakeup`, `/sys/power/wakeup_count`).
⚠️ **devices/lid** -- functionally complete but **poll-based**, not
event-based: reads `/proc/acpi/button/lid` on the daemon's regular poll
tick (a few seconds of latency by default), rather than watching a live
evdev `SW_LID` switch. Actually works end to end; just not instant.
‼️ **devices/power_button, keyboard** -- the decision logic (short/long/
multi-press classification; brightness step math) is fully implemented and
unit tested. **The live hardware event source is not implemented** -- no
evdev `KEY_POWER`/`KEY_BRIGHTNESSUP` watcher exists, so nothing currently
calls this logic. I deliberately did not fake this with uevents (uevents
signal device hotplug, not individual keypresses, so routing through them
would have been actively misleading). `apply()`/`classify()`/`step_up()`/
`step_down()` are ready for a real evdev integration to call.

## 4. Policy

✅ **battery_policy, thermal_policy, profile_policy** -- complete, wired
into `PolicyEngine`, unit tested.
✅ **lid_policy** -- complete, unit tested, wired into `devices::lid`.
⚠️ **idle_policy** -- currently a thin passthrough to `idle::policy`; the
AC-vs-battery differentiation it's structured to hold isn't populated with
any actual difference yet (both currently get the same timeouts).
⚠️ **sleep_policy** -- implemented (`choose()`) but **not called from
anywhere**. Clients pick a sleep method explicitly via the specific IPC
method today; nothing currently asks "what should I do" and gets an
automatic answer.
⚠️ **thermal.critical_action config key** -- parsed and exposed, but
**intentionally never consulted**: Critical always triggers emergency
shutdown regardless of this value (a deliberate safety decision so a
misconfigured `thermal.toml` can't disable the safety net -- see
`src/config/defaults.rs` and `src/thermal/protection.rs`).

## 5. IPC

✅ **protocol, events, messages, permissions, server, client** -- complete.
All 30 methods dispatch; all 18 events are emitted from somewhere in the
codebase except `PowerButtonPressed` (blocked on the gap in section 3
above). Unit tested (protocol round-trips) and integration tested (a real
client against a real server over a real Unix socket, `tests/ipc.rs`).
⚠️ **"Session" permission tier** -- currently identical to Public (no
restriction beyond "opened the socket"). The intended "restricted to the
active seat's session user" semantics need session/seat tracking that
doesn't exist in mitos-power (that's mitos-session's domain) -- see
docs/security.md.
⚠️ **`InhibitMode::Delay`** -- parsed and stored, but v0 treats it
identically to `Block`. The intended "give the holder a grace period, then
proceed anyway" semantics aren't implemented -- see docs/power-model.md.

## 6. Persistence / monitoring / logging

⚠️ **persistence** (settings, statistics, history) -- fully implemented and
unit tested as a standalone module (atomic JSON-file writes, capped
history). **Not called from `Daemon`/`PowerManager` anywhere** -- nothing
persists across a restart today. Wiring it in (load on `Daemon::new`, save
on the relevant state changes) is the natural next step.
⚠️ **monitoring** (metrics counters, in-memory event log, diagnostics) --
same situation: implemented and usable, but `Metrics::incr_*` is never
called from the live request path, so every counter would read zero.
⚠️ **logging::audit** -- real, and actually wired in (every privileged
action logs through it). Rides on regular `tracing` output at the "audit"
target rather than a dedicated rotated log file -- see docs/security.md.

## 7. Hardening

‼️ **No capability dropping.** The daemon needs root today (raw reboot
syscalls, sysfs writes under `/sys/power`, `/sys/class/backlight`, etc.).
Narrowing to `CAP_SYS_BOOT` + specific sysfs write access, and adding
systemd sandboxing directives (`ProtectSystem`, etc.) to
`services/mitos-power.service`, is not done.
‼️ **No swap-sizing validation before hibernate.** `Hibernate`/`HybridSleep`
attempt the write and surface whatever the kernel reports (`UNSUPPORTED` if
`/sys/power/state` doesn't list `disk`); they don't proactively check swap
size vs. RAM or `resume=` before trying.

## 8. Tests

✅ Unit tests live next to the code for: sysfs helpers, battery math/parsing,
IPC protocol serialization, inhibitor manager, idle detector, cpu core-range
parsing, config validation, persistence database round-trip, lid/battery
policy, power-button/brightness classification.
✅ Integration tests (`tests/`) cover: battery/AC consistency, brightness
controller on a no-backlight machine, profile switching, thermal manager
construction, idle transitions, inhibitor categories, a **real** end-to-end
IPC round-trip over an actual Unix socket, and permission-tier
classification.
⚠️ Deliberately `#[ignore]`d: every test that would actually suspend,
hibernate, reboot, or power off the machine running `cargo test` --
running those safely requires a disposable VM (`cargo test -- --ignored`
there, never in normal CI).
‼️ No coverage at all for: `cpu::frequency`/`topology` (trivial reads, low
risk), `thermal::throttling`, `devices::power_button`/`keyboard`'s
still-unwired event source (nothing to test yet), `persistence`/
`monitoring` wiring (because there is none yet -- see section 6), the
`daemon::event_loop` select! loop itself (would need a much heavier test
harness to exercise realistically).
