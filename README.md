# mitos-power

The central power-management daemon for MITOS: battery, charging, AC,
suspend/hibernate, shutdown/reboot, display power and brightness, thermal
monitoring, CPU power profiles, idle behavior, and power inhibitors -- all
behind one Unix-socket IPC API that every other mitos-* component talks to.

**Status: first-pass implementation, never compiled.** This was written in
an environment with no Rust toolchain and no network access, so nothing
here has been through `cargo build`. See **[audit.md](audit.md)** for an
honest, itemized list of what's fully implemented vs. what's a documented
gap -- read that before assuming any given piece works.

## Building

```
cd mitos-power
cargo build --release
```

Two binaries come out of this: `mitos-power` (the daemon) and
`mitos-powerctl` (the CLI). See [audit.md](audit.md) section 0 for the two
spots most likely to need a small fix on first compile (`nix` crate feature
flags, `udev` crate API shape).

## Running

```
sudo ./target/release/mitos-power --config-dir ./config
```

mitos-power needs root (suspend/reboot syscalls, sysfs writes) --see
[docs/security.md](docs/security.md). A systemd unit is provided at
`services/mitos-power.service` for development/testing; MITOS's own service
manager (mitos-services) is the intended production target.

Config lives in six independently-optional TOML files (defaults shown in
`config/*.toml` at the repo root); missing files, or missing individual keys
within a present file, fall back to built-in defaults -- see
`src/config/defaults.rs`.

## Using it

```
mitos-powerctl status
mitos-powerctl battery
mitos-powerctl profile performance
mitos-powerctl brightness 60
mitos-powerctl suspend
mitos-powerctl inhibitors
mitos-powerctl thermal
```

Every other mitos-* component (mitos-gui, mitos-session, mitos-settings,
applications) talks to the same socket `mitos-powerctl` does --
**[docs/ipc.md](docs/ipc.md) is the complete reference**: every method,
every event, the permission model, and the exact wire format.

## Documentation

| File | Covers |
|---|---|
| [docs/ipc.md](docs/ipc.md) | **The full IPC/API reference** -- every endpoint, params, results, events, permissions |
| [docs/architecture.md](docs/architecture.md) | Module layout, the policy engine, lock discipline, config reload |
| [docs/power-model.md](docs/power-model.md) | How battery/profile/thermal/idle numbers are derived |
| [docs/security.md](docs/security.md) | Trust boundary, permission tiers, running as root |
| [docs/sleep.md](docs/sleep.md) | Suspend/hibernate/hybrid-sleep/suspend-then-hibernate in detail |
| [docs/troubleshooting.md](docs/troubleshooting.md) | Common problems and what they mean |
| [audit.md](audit.md) | What's real vs. what's a documented gap |

## Notes on structure

This follows the spec's `src/` layout exactly, with one addition:
`src/lib.rs`. The original structure listed only `src/main.rs`, but with two
binaries (`mitos-power`, `mitos-powerctl`) sharing the IPC protocol, a
library crate is the standard way to guarantee both are built from the
identical Rust types rather than two independently-maintained copies of the
wire format. `src/main.rs` and `bin/mitos-powerctl.rs` are now both thin
wrappers around `mitos_power::*`.

## License

MIT -- see [LICENSE](LICENSE).
