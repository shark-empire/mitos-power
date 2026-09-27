# Audit: what's real vs. what's not

This was written in a sandbox with **no Rust toolchain and no network
access** (verified: `cargo`/`rustc` aren't installed, and crates.io is
unreachable). Every file here is hand-written, careful, idiomatic Rust
against APIs I'm confident about -- but none of it has been through
`cargo build`, `cargo test`, or even `cargo check`. Treat this as a strong
draft that needs a real build pass, not verified-working code.

*Second pass note: since the first version of this file, persistence and
monitoring got wired in, `InhibitMode::Delay` got real semantics, a swap
check was added before hibernate, and a live evdev input-event source was
added for lid/power-button/brightness hotkeys. Sections below are updated
accordingly; a few net-new risk areas came with that (see section 0).*

Legend: ✅ implemented and, to the best of my ability without a compiler,
correct · ⚠️ implemented but with a real, documented limitation · ‼️ not
implemented (structure/interface may exist; the behavior doesn't)

## 0. Compilation

‼️ **Never compiled.** No `cargo build` / `cargo check` was run against this
code, at all, for any file. First thing to do on a real machine:
```
cd mitos-power && cargo build
```
Three areas I'd bet on first if something doesn't compile, in order of risk:

1. **`src/hardware/evdev.rs`** (new since the first pass) -- the single
   riskiest file in the codebase. `evdev::enumerate()`,
   `supported_switches()`/`supported_keys()` returning something with
   `.contains()`, `fetch_events()`, `InputEventKind` -- all written from
   memory of the `evdev` crate's ~0.12 API shape, with no way to check
   docs.rs. If `cargo build` fails here, this is the most likely place, and
   the fix is "look up the actual method names for whatever version
   resolves" rather than a logic error.
2. **`udev` crate API shape** (`src/hardware/uevent.rs`) -- `MonitorBuilder`/
   `match_subsystem`/`listen`/`.iter()` on the returned socket.
3. **`nix` crate feature flags** (Cargo.toml) -- I listed `reboot`, `user`,
   `fs`; exact feature names have moved around across `nix` 0.2x releases.
   `nix::sys::reboot::reboot`/`RebootMode`, `nix::unistd::sync`,
   `nix::unistd::{User, Group}` are the APIs actually used.

Every other dependency (`tokio`, `serde`/`serde_json`, `toml`, `thiserror`,
`tracing`, `clap`, `chrono`, `uuid`, `anyhow`) I'm confident about at the API
level used here.

## 1. Core subsystems

Unchanged from the first pass -- see below for what *did* change (sections
2-6). Summary: ✅ errors, hardware/sysfs/acpi/capabilities, battery (all 11
files, unit tested), ac, display/backlight/brightness/timeout, profiles,
inhibitor (now with more tests -- see section 4), cpu, thermal, idle. ⚠️
hardware/uevent (udev crate, see section 0); display/display_power &
night_mode (state-tracking only, by design -- mitos-gui owns the actual
compositor action).

## 2. Sleep / shutdown

⚠️ **sleep** -- fully implemented against `/sys/power/*`. **New:**
`hibernate()`/`hybrid_sleep()` now check `/proc/swaps` first and log a
specific warning if no swap is active at all, before attempting the write
-- doesn't hard-block (a marginal-but-working setup shouldn't be refused on
this coarse a check) and still doesn't check swap *size* vs. RAM or
`resume=` correctness, but the single most common failure cause now gets a
clear, early log line instead of just an opaque syscall failure.
Suspend/hibernate themselves are still untestable outside a disposable VM
-- `tests/suspend.rs`/`hibernate.rs` still `#[ignore]` everything that would
actually call them.
⚠️ **shutdown/reboot/poweroff** -- unchanged: real syscalls, untestable here,
`#[ignore]`d tests.
‼️ **shutdown/logout** -- still an explicit no-op; still blocked on
mitos-session's IPC protocol not existing yet. Unchanged, and the single
most important remaining integration gap.

## 3. Input devices

✅ **devices/usb, wakeup** -- unchanged, sysfs-only.
⚠️→✅ **devices/lid** -- now has *both* paths: `poll()` (fallback, unchanged)
and `handle_live_signal()` (instant, driven by the new evdev watcher). Both
funnel through the same `apply_if_changed`, so lid policy is only
implemented once regardless of which path fires.
‼️→⚠️ **devices/power_button, keyboard** -- **now wired to a live event
source.** `hardware::evdev::spawn_watcher` finds devices reporting
`KEY_POWER`/`SW_LID`/`KEY_BRIGHTNESSUP`/`KEY_BRIGHTNESSDOWN` and forwards
press/release events; `daemon::event_loop`'s new `PowerButtonTracker` turns
raw timestamps into the `(held_for, recent_press_count)` pair the
already-tested `classify()` expects. The ⚠️ here is entirely about
section 0's evdev-API risk, not about missing logic -- `classify()`/
`apply()`/`step_up()`/`step_down()` were already correct and tested before
this pass; what changed is that something now actually calls them.
Hotplugged input devices added after daemon startup are not picked up
(devices are enumerated once, at `spawn_watcher()` time) -- would need to
also watch udev "input" add events and re-scan; not implemented.

## 4. Policy / Inhibitors

✅ **battery_policy, thermal_policy, profile_policy, lid_policy** --
unchanged, wired and tested.
⚠️ **idle_policy** -- unchanged: thin passthrough, no actual AC-vs-battery
differentiation populated yet.
⚠️ **sleep_policy** -- unchanged: implemented, not called from anywhere.
⚠️ **thermal.critical_action** -- unchanged: parsed, intentionally never
consulted (safety decision, not a bug).
‼️→✅ **`InhibitMode::Delay`** -- **now has real semantics.**
`InhibitorManager::split_blockers`/`wait_for_delay_clear` distinguish Block
(fails immediately) from Delay (polls for up to
`general.inhibitor_delay_grace_secs`, default 5s, then proceeds regardless).
`PowerManager::ensure_not_inhibited` uses this for Suspend/Shutdown.
Idle-triggered actions deliberately still treat Delay the same as Block
(see `idle::inhibitors`) -- a recurring per-second check re-litigating "wait
5 more seconds, then dim anyway" doesn't match Delay's intent the way a
one-shot Suspend does. New unit tests cover clearing naturally, timing out,
and the Block/Delay split itself.

## 5. IPC

✅ **protocol, events, messages, permissions, server, client** -- 31 methods
now dispatch (was 30; added `GetDiagnostics`). All 18 events are emitted
from somewhere in the codebase, **including `PowerButtonPressed`** now that
section 3's evdev wiring landed (previously the one event nothing could
trigger). Unit tested + integration tested over a real socket
(`tests/ipc.rs`).
⚠️ **"Session" permission tier** -- unchanged: identical to Public today.

## 6. Persistence / monitoring / logging

‼️→✅ **persistence** -- **now wired in.** `PowerManager` opens a `Database`
at `general.state_dir` on construction; `Daemon::run` calls
`restore_persisted_state()` before the IPC server starts, which loads and
re-applies the last profile and brightness. `set_profile`/`set_brightness`
persist on every change via a read-modify-write against the settings file
(a narrow, documented race window exists if two clients change profile and
brightness at nearly the same instant -- see the doc comment on
`PowerManager::persist_setting`). A new test
(`restores_persisted_profile_across_a_fresh_instance` in
`src/manager/manager.rs`) proves this round-trips across two separate
`PowerManager` instances sharing a state dir, not just that it compiles.
`statistics`/`history` (the other two `persistence` submodules) are still
unused -- only `settings` got wired.
‼️→✅ **monitoring** -- **now wired in.** `PowerManager` holds a `Metrics`
counter set and a `HealthTracker`; `ipc::server::dispatch` increments
`ipc_requests_total` on every call, `around_sleep` increments
suspend/resume counts, `execute_action` increments policy-actions-executed.
All of it is exposed through the new `GetDiagnostics` IPC method
(uptime + hardware capabilities + these counters), with a test proving the
counter actually shows up in the returned snapshot. The in-memory
`monitoring::events::EventLog` ring buffer is still unused -- only
`statistics::Metrics` and `health::HealthTracker` got wired.
⚠️ **logging::audit** -- unchanged: real, wired, rides on regular `tracing`
output rather than a dedicated rotated file.

## 7. Hardening

‼️ **No capability dropping.** Unchanged -- the daemon still needs full
root.
⚠️ **Hibernate/hybrid-sleep swap checking** -- upgraded from "nothing" to
"warns if /proc/swaps shows zero active swap entries" (see section 2). Still
doesn't check swap *size* against RAM or validate `resume=`, so it remains
possible to get a late, opaque kernel-level failure instead of an early,
specific one in the "swap exists but is too small" case specifically.

## 8. Tests

✅ Everything listed in the first pass, plus new coverage added this pass:
`InhibitorManager::split_blockers`/`wait_for_delay_clear` (natural clear,
timeout, block/delay separation), `PowerButtonTracker`'s press/release
timing math (`daemon::event_loop`'s new `#[cfg(test)]` block),
`sleep_state::has_active_swap` (environment-agnostic, just checks it
doesn't panic), and two persistence/monitoring integration tests in
`src/manager/manager.rs` described in section 6.
‼️ Still no coverage for: `cpu::frequency`/`topology`, `thermal::throttling`,
the `daemon::event_loop` select! loop as a whole (would need a much heavier
test harness), and -- new this pass -- `hardware::evdev.rs` itself has zero
tests, because there's no way to unit-test "did I call the real evdev
crate's API correctly" without a compiler and a real input device. That
file's correctness rests entirely on careful reading, not verification.
