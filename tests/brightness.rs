//! Brightness step math and controller behavior on a machine with no
//! backlight device (this sandbox and most CI containers). Setting
//! brightness is non-destructive, so unlike suspend/shutdown these run
//! unconditionally, even on a machine that does have a real backlight.

use mitos_power::devices::keyboard::{step_down, step_up};
use mitos_power::display::BrightnessController;

#[test]
fn step_functions_clamp_at_bounds() {
    assert_eq!(step_up(100), 100);
    assert_eq!(step_down(0), 0);
}

#[test]
fn brightness_controller_construction_never_fails() {
    // Even with zero backlight devices present, constructing the
    // controller should succeed -- `is_supported()` is how callers check
    // for a real device, not a constructor error.
    let controller = BrightnessController::new().expect("BrightnessController::new should not fail even with no backlight device");
    if !controller.is_supported() {
        assert!(controller.get_percent().is_err());
        assert!(controller.set_percent(50).is_err());
    }
}
