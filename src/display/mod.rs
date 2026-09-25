//! Display power: backlight brightness (owned directly by mitos-power via
//! sysfs) and screen blanking/night-mode (delegated to mitos-gui, which
//! owns the compositor and therefore the actual DRM/KMS output state --
//! see docs/ipc.md "Display power ownership").

pub mod backlight;
pub mod brightness;
pub mod display_power;
pub mod night_mode;
pub mod timeout;

pub use brightness::BrightnessController;
pub use display_power::{DisplayPowerController, DisplayPowerState};
