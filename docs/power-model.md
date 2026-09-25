# Power model

## Battery

Each `BAT*` under `/sys/class/power_supply` is read into a `BatteryDevice`,
then converted to the public `BatteryInfo` (what IPC clients see):

- **Percentage**: `energy_now / energy_full`, falling back to `charge_now /
  charge_full`, falling back to the kernel's own `capacity` file --
  whichever the fuel gauge actually exposes.
- **Health**: `energy_full / energy_full_design` (or the charge-based
  equivalent). `None` if the driver doesn't expose design capacity at all.
- **Time remaining**: present energy divided by instantaneous power draw.
  An estimate, not a promise -- it moves as load changes.
- **Overall percentage** (multi-battery machines): a simple mean across every
  present battery.

## AC

Any `power_supply` entry whose `type` attribute is `"Mains"` counts as an AC
adapter (not name-matching "AC"/"ADP0"/etc., which varies by vendor). "On AC"
is true if *any* discovered adapter reports online.

## Profiles

| Profile | Governor | Boost | Display timeout | Max brightness | Background |
|---|---|---|---|---|---|
| performance | performance | on | 900s | 100% | not throttled |
| balanced | schedutil | on | 600s | 100% | not throttled |
| powersave | powersave | off | 180s | 60% | throttled |

Custom profiles come from `profiles.toml`'s `[[profile]]` entries and carry
the same fields. Switching profiles applies the CPU governor/boost
immediately and recomputes the idle detector's dim/off timeouts (see
`docs/architecture.md` "Policy engine").

## Thermal

Highest reading across every `/sys/class/thermal/thermal_zone*` is compared
against `thermal.toml`'s `warning_temp_c`/`critical_temp_c`. Warning drops the
profile to powersave; Critical triggers an immediate emergency shutdown.
mitos-power never writes kernel trip points and never tries to out-throttle
the kernel's own thermal driver, which remains responsible for fundamental
hardware safety regardless.

## Idle / display tiers

```
0 ---- dim_after ---- off_after ---- (optional) suspend_timeout_secs
Awake       Dim            Off              Suspended
```

`dim_after`/`off_after` come from the active profile's `display_timeout_secs`
(clamped so dim always fires before off); auto-suspend-on-idle is a separate,
opt-in, typically much longer threshold (`display.suspend_on_idle` +
`suspend_timeout_secs`). An `Idle` inhibitor blocks all three tiers -- see
`docs/ipc.md` "Inhibitors".

## Inhibitors

Free-form `(who, why, what, mode)` leases, auto-released when the holding
IPC connection closes. `what` is `suspend | shutdown | idle | all`; `mode` is
`block | delay` (v0 treats both identically -- see audit.md for the planned
grace-period semantics of `delay`).
