//! Input-adjacent devices: lid switch, power button, brightness hotkeys,
//! and wakeup-source management for USB and other buses.
//!
//! Coverage note: `lid` is fully implemented via polling. `power_button`
//! and `keyboard` have their decision logic fully implemented and unit
//! tested, but are not yet wired to a live evdev event source -- see
//! audit.md for exactly what that means. `usb` and `wakeup` are fully
//! implemented (sysfs-only, no evdev needed).

pub mod keyboard;
pub mod lid;
pub mod power_button;
pub mod usb;
pub mod wakeup;
