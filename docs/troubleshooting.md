# Troubleshooting

**`mitos-powerctl` says "could not connect to mitos-power ... is the daemon
running?"**
Check the daemon is actually running (`systemctl status mitos-power` if
using the provided unit) and that `/run/mitos/power.sock` exists. If it
exists but you still get a permission error, confirm your user is in the
`power` group (`security.privileged_group` in `power.toml`) or run
`mitos-powerctl` as root.

**`Suspend`/`Hibernate` return an `INHIBITED` error**
Something is holding a `suspend`- or `all`-scoped inhibitor. `mitos-powerctl
inhibitors` lists who and why. Pass `force: true` on `Shutdown`/`Reboot` to
bypass a `Shutdown`-scoped inhibitor (there is no force override for
Suspend/Hibernate today -- release the inhibitor instead).

**Hibernate fails with `UNSUPPORTED`**
The kernel's `/sys/power/state` doesn't list `disk`. Usually means no swap,
swap smaller than RAM, or missing `resume=<device>` on the kernel command
line. mitos-power does not check swap sizing itself before attempting the
write -- see audit.md.

**Brightness calls return `UNSUPPORTED`**
No `/sys/class/backlight/*` device was found at daemon startup. Common on
desktops/VMs (no backlight to control) and on some external-only-display
laptops depending on driver support. `mitos-powerctl status` won't show a
brightness line in this case either.

**Lid close/open isn't detected instantly**
Lid state is polled (via `/proc/acpi/button/lid`) on the daemon's regular
poll tick, not watched live via evdev -- expect up to `poll_interval_secs`
of latency (a few seconds by default). See audit.md for what a live evdev
watcher would add.

**Power button / brightness hotkeys don't do anything**
These aren't wired to a live input event source yet -- the decision logic
(`devices::power_button`, `devices::keyboard`) is implemented and tested,
but nothing currently calls it. See audit.md.

**A setting in `power.toml` doesn't seem to apply after I sent SIGHUP**
Some config is cached at startup (the profile list, idle timeouts) and needs
a full daemon restart to pick up changes, not just a reload. See
`docs/architecture.md` "Config reload" for exactly which fields are and
aren't live-reloadable.
