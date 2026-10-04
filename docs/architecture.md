# mitos-power architecture

## Layout

```
main.rs / bin/mitos-powerctl.rs   thin entry points
lib.rs                            re-exports every subsystem below
daemon/        wiring: construct PowerManager, run the IPC server + event loop, handle signals
manager/       PowerManager (central orchestrator) + PolicyEngine + PowerStateSnapshot + Scheduler
battery/ ac/ display/ cpu/ thermal/ idle/ profiles/ devices/ inhibitor/   subsystems
hardware/      sysfs/acpi/udev access -- the only code that touches /sys or /proc directly
ipc/           wire protocol, Unix-socket server + client, permissions
policy/        battery/thermal/profile/lid/idle/sleep decision logic
persistence/ monitoring/ logging/ config/ errors/   support modules
```

Every hardware read/write goes through `hardware::sysfs` (or `hardware::acpi`/`hardware::uevent`).
Nothing outside `hardware/` touches `/sys` or `/proc` paths directly -- that keeps
"this file doesn't exist on some laptops" handled in exactly one place.

## PowerManager

`manager::manager::PowerManager` owns every subsystem manager (`battery`, `ac`,
`profiles`, `thermal`, `inhibitors`, `brightness`, `cpu`, `idle`, ...) behind
`tokio::sync::RwLock`s, plus a `broadcast::Sender<DaemonEvent>` that both the
IPC server and internal callers publish through. It is the single object the
IPC dispatcher (`ipc::server::dispatch_inner`) calls into -- one `PowerManager`
method maps to (usually) exactly one IPC method. See `docs/ipc.md` for the
wire-level view of this same surface.

Lock discipline: every write-lock acquisition in `PowerManager` is scoped to a
single statement (never held across an `.await` that could acquire a second
lock), so there's no lock-ordering cycle across the whole manager. The one
place multiple locks are held simultaneously is `get_power_state()`, and
those are all read locks, which don't conflict with each other.

## Policy engine

Three policies (`battery_policy`, `thermal_policy`, `profile_policy`) implement
`manager::policy::Policy` and run through `PolicyEngine::evaluate()` on every
poll tick: `(PowerStateSnapshot, Config) -> Vec<Action>`, then
`PowerManager::execute_action` carries each one out. This is the mechanism
behind the spec's "policy-based, not hardcoded" requirement -- e.g. a profile
switch changes `idle::detector`'s dim/off timeouts via
`idle::policy::effective_timeouts`, driven by `profiles.toml`, not an
if/else chain.

Two policies are *not* run through `PolicyEngine`:

- `lid_policy` is evaluated once, directly, when `devices::lid::poll` observes
  the lid state change -- there's no reason to wait for the next scheduled
  tick to react to a lid closing.
- `idle_policy` / `idle::policy` likewise run from `daemon::event_loop`'s
  per-second idle tick, not the (slower) hardware poll tick.

`thermal_policy` itself only ever sees the *Warning* tier. Critical is handled
immediately, synchronously, at refresh time by `thermal::protection` --
an emergency shutdown must not wait for the next scheduled policy pass.

`sleep_policy` is an advisory helper (`choose()`), not an independent action
producer; nothing currently calls it automatically (see audit.md).

## Suspend/resume and the recursion hazard

`PowerManager::refresh_and_run_policy()` = refresh hardware state, then run
the policy engine. `sleep::wake::on_resume` (called after every suspend/
hibernate/hybrid-sleep/suspend-then-hibernate) deliberately calls only the
first half (`refresh_hardware_state`), **not** the policy tick. If it ran the
full policy tick and the battery was still critical immediately after waking
from a policy-triggered hibernate, the policy would fire hibernate again,
wake, fire again, without bound. The regular poll timer runs the policy
engine again a few seconds later regardless, so nothing is lost by skipping
it specifically on the resume path.

## Display power ownership

mitos-power owns backlight *brightness* directly (`display::brightness`,
plain sysfs) because that's simple, standard, and hardware-level. It does
**not** own screen blanking/DPMS or night-mode color temperature -- those
require compositor cooperation, so mitos-power only tracks the *intended*
state (`display::display_power::DisplayPowerController`) and emits
`IdleStateChanged` / relies on mitos-gui to subscribe and perform the actual
DRM/KMS output power change. See `docs/ipc.md`.

## Config reload

SIGHUP reloads `power.toml`/`profiles.toml`/etc. from disk and replaces the
shared `RwLock<Config>` in place. Everything that reads config fresh on each
use (the policy engine, IPC permission checks, sleep timing) picks this up
immediately. Subsystems that cached a value at construction time -- the
profile list built in `ProfileManager::new`, the idle detector's timeouts set
in `IdleDetector::new` -- do **not** get rebuilt by a reload; those need a
daemon restart today. Full hot-reload of every subsystem is a natural
follow-up (see audit.md).

## Integration

```
                 Linux Kernel
                     |
          +----------+----------+
          |          |          |
        ACPI       sysfs       udev
          |          |          |
          +----------+----------+
                     |
                mitos-power  <----------------+
                     |                        |
       +-------------+-------------+          | Suspend/Reboot/PowerOff
       |             |             |          | (see below)
mitos-session   mitos-services  mitos-gui      |
       |             |                         |
       +-------------+-------------------------+
                     |
              mitos-settings
```

mitos-gui/mitos-settings/mitos-services are (or would be) plain clients of
mitos-power's IPC socket, as `docs/ipc.md` describes. mitos-session is
different: it's the one component mitos-power also calls *out* to.

### mitos-session integration

**The problem this solves.** Reading mitos-session's actual source (its
`power/` module) surfaced a real conflict: mitos-session shipped with its
own `SystemPowerBackend`, writing to `/sys/power/state` and calling
`reboot(2)` directly -- completely independent of mitos-power, which the
original spec already named as the sole owner of exactly those operations.
Two daemons both able to trigger a kernel-level suspend/reboot/poweroff,
with only one of them (mitos-session) also locking sessions and notifying
compositors first, is a real bug waiting to happen: anything that reached
mitos-power directly -- the policy engine's auto-hibernate on critical
battery, a lid close, `mitos-powerctl suspend`, an app that simply didn't
know to prefer mitos-session -- would suspend the machine with the desktop
left unlocked.

**The resolution**, implemented across both projects:

- mitos-session's `power` module gained a `mitos_power` backend
  (`session_client`'s counterpart on that side) that calls mitos-power's
  `Suspend`/`Reboot`/`PowerOff` instead of touching the kernel itself. It
  remains the default there. mitos-session's own inhibitor-check,
  session-locking and compositor-notification sequence is unchanged --
  only the final kernel transition moved. mitos-session's `Direct`
  backend still exists for a mitos-power-less boot.
- Symmetrically, mitos-power gained `session_client` (`src/session_client/`,
  not part of the original spec's file tree): an async client for
  mitos-session's real wire protocol -- length-prefixed bincode, a
  completely different format from mitos-power's own NDJSON, mirrored
  by hand in `session_client::protocol` since the two are independent
  Cargo projects with no shared crate. `PowerManager` now calls it in
  three places:
  - `Logout` ends the caller's mitos-session sessions (`shutdown::logout`)
    -- previously a documented no-op.
  - `around_sleep` (Suspend/Hibernate/HybridSleep/SuspendThenHibernate)
    locks every session first (`notify_session_before_sleep`).
  - `shutdown`/`reboot`/`poweroff` wind sessions down first
    (`notify_session_before_shutdown`).

**Avoiding a call cycle.** mitos-session's `Suspend` request already locks
sessions before calling mitos-power; mitos-power's own `Suspend` *also*
tries to lock sessions before proceeding. When the call came from
mitos-session, this is redundant -- but harmless, since locking an
already-locked session is a no-op on mitos-session's side (and
`session_client` skips already-locked sessions itself as a further guard).
Neither side waits for the other to finish its own lock/notify sequence
before doing its part; there's no shared "I'm already handling this,
don't do it again" handshake. That's a deliberate simplicity trade-off,
not an oversight -- see audit.md for what a tighter handshake would need.

**Honestly-scoped limits, by design, not oversight:**
- Every `session_client` call is best-effort: if mitos-session is
  unreachable (a legitimate minimal-boot configuration, per its own
  README), locking/winding-down silently does nothing and the power
  action proceeds anyway. `Logout` is the one exception -- it's the
  entire point of the call, so an unreachable mitos-session is a real
  error there, not a silent no-op.
- mitos-power's pre-sleep lock reaches every session's *lock state*, not
  its compositor notification (`PrepareForSleep`/`ResumedFromSleep`).
  Those events are internal to mitos-session's own `Suspend` handler and
  aren't triggered by locking a session from outside. A compositor that
  needs to know sleep is imminent (to pause video, say) only reliably
  gets that when the request came through mitos-session in the first
  place.
- `session_client::protocol`'s types were mirrored by reading
  mitos-session's source once; there is no shared crate and no compiler
  check linking the two. If mitos-session's wire types change, this
  drifts out of sync silently. See audit.md.
