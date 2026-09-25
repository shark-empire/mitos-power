//! Low-level hardware interfaces. Every other subsystem (battery, ac,
//! display, cpu, thermal) reads/writes hardware exclusively through this
//! module -- nothing outside `hardware/` should touch `/sys` or `/proc`
//! directly. That keeps "this file doesn't exist on some laptops" handled
//! in exactly one place.

pub mod acpi;
pub mod capabilities;
pub mod sysfs;
pub mod uevent;

pub use capabilities::HardwareCapabilities;
