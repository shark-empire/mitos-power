//! Raw "time since last activity" tracking, kept decoupled from tier
//! classification (see `detector.rs`) so each half is independently
//! testable.

use std::time::{Duration, Instant};

pub struct ActivityTimer {
    last_activity: Instant,
}

impl ActivityTimer {
    pub fn new() -> Self {
        Self { last_activity: Instant::now() }
    }

    pub fn reset(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn elapsed(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

impl Default for ActivityTimer {
    fn default() -> Self {
        Self::new()
    }
}
