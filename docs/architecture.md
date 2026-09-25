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
                mitos-power
                     |
       +-------------+-------------+
       |             |             |
mitos-session   mitos-services  mitos-gui
       |             |             |
       +-------------+-------------+
                     |
              mitos-settings
```

mitos-power never talks to mitos-session/mitos-gui/mitos-settings/mitos-services
directly -- every one of those is a *client* of mitos-power's IPC socket (see
`docs/ipc.md`), and mitos-power has no outbound client of its own to any of
them yet (`shutdown::logout` documents this gap explicitly for the one place
it matters most today).
