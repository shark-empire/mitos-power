//! Input-adjacent devices: lid switch, power button, brightness hotkeys,
//! and wakeup-source management for USB and other buses.
//!
//! Coverage note: `lid` works both via polling (`poll`, always available)
//! and instantly via evdev (`handle_live_signal`, when
//! `hardware::evdev::spawn_watcher` finds a device). `power_button` and
//! `keyboard` now have a live evdev-backed event source too (see
//! `daemon::event_loop`) -- their decision/step logic was already unit
//! tested standalone; audit.md has the specifics of what's
//! verified-by-inspection-only vs. hardware-tested. `usb` and `wakeup` are
//! fully implemented (sysfs-only, no evdev needed).

pub mod keyboard;
pub mod lid;
pub mod power_button;
pub mod usb;
pub mod wakeup;
