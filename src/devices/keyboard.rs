//! Brightness hotkey handling (KEY_BRIGHTNESSUP / KEY_BRIGHTNESSDOWN).
//!
//! `step_up`/`step_down` are pure and unit tested. The live event source
//! is `hardware::evdev::spawn_watcher`, consumed directly by
//! `daemon::event_loop` (no extra state needed here, unlike power_button's
//! press-timing tracker -- each hotkey press is a complete signal on its
//! own).

use crate::manager::PowerManager;
use std::sync::Arc;

const STEP_PERCENT: u8 = 10;

pub fn step_up(current: u8) -> u8 {
    current.saturating_add(STEP_PERCENT).min(100)
}

pub fn step_down(current: u8) -> u8 {
    current.saturating_sub(STEP_PERCENT)
}

pub async fn handle_brightness_up(manager: &Arc<PowerManager>) {
    if let Ok(current) = manager.get_brightness().await {
        let _ = manager.set_brightness(step_up(current)).await;
    }
}

pub async fn handle_brightness_down(manager: &Arc<PowerManager>) {
    if let Ok(current) = manager.get_brightness().await {
        let _ = manager.set_brightness(step_down(current)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_up_clamps_at_100() {
        assert_eq!(step_up(95), 100);
        assert_eq!(step_up(50), 60);
    }

    #[test]
    fn step_down_clamps_at_0() {
        assert_eq!(step_down(5), 0);
        assert_eq!(step_down(50), 40);
    }
}
