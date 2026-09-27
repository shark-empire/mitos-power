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
line. mitos-power logs a warning (not a hard block) if `/proc/swaps` shows
no active swap at all before even attempting the write, since that's the
single most common cause -- check the daemon log for that line first. It
does not check swap *size* vs. RAM or `resume=` correctness, so a kernel
that lists `disk` as supported but is still misconfigured will fail at the
syscall itself rather than being caught early.

**Brightness calls return `UNSUPPORTED`**
No `/sys/class/backlight/*` device was found at daemon startup. Common on
desktops/VMs (no backlight to control) and on some external-only-display
laptops depending on driver support. `mitos-powerctl status` won't show a
brightness line in this case either.

**Lid close/open takes a few seconds to register**
This means the evdev watcher didn't find a `SW_LID`-reporting device at
startup (check the daemon log), so mitos-power fell back to polling
`/proc/acpi/button/lid` on the regular poll tick -- expect up to
`poll_interval_secs` of latency in that fallback mode (a few seconds by
default). With the evdev watcher active, lid changes are instant.

**Power button / brightness hotkeys don't do anything**
Check the daemon log at startup for the evdev watcher line -- it logs
either "evdev input watcher active" or a specific reason it couldn't find
a matching device (most likely cause: the daemon isn't running with
permission to read `/dev/input/event*`, which normally means membership in
the `input` group or running as root). This is also the single least
hardware-tested part of mitos-power -- see audit.md section 0 and the
`src/hardware/evdev.rs` header comment if it's failing in a way that looks
like a wrong API call rather than a permissions issue.

**A setting in `power.toml` doesn't seem to apply after I sent SIGHUP**
Some config is cached at startup (the profile list, idle timeouts) and needs
a full daemon restart to pick up changes, not just a reload. See
`docs/architecture.md` "Config reload" for exactly which fields are and
aren't live-reloadable.
