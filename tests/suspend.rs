//! Suspend/hibernate/hybrid-sleep/suspend-then-hibernate genuinely cannot
//! be safely exercised in an automated test -- doing so would suspend (or
//! power off) whatever machine runs `cargo test`. Anything that calls the
//! real sleep::* functions is `#[ignore]`d; run with `cargo test --
//! --ignored` only on a disposable VM you don't mind actually sleeping.
//!
//! What IS safely tested here: capability probing never panics, on a
//! machine that has real /sys/power/* files or (as in this sandbox) none
//! at all.

use mitos_power::hardware::capabilities::HardwareCapabilities;

#[test]
fn capability_detection_does_not_panic_without_sys_power() {
    let caps = HardwareCapabilities::detect();
    // No specific value assertion -- the interesting outcome is that
    // detection completed at all rather than panicking on a missing path.
    let _ = caps.suspend;
    let _ = caps.hibernate;
}

#[tokio::test]
#[ignore = "suspends the machine running the test if suspend is actually supported -- only run deliberately on a disposable VM"]
async fn suspend_actually_suspends() {
    let _ = mitos_power::sleep::suspend::suspend().await;
}

#[tokio::test]
#[ignore = "suspends (and possibly hibernates) the machine running the test -- only run deliberately on a disposable VM"]
async fn hybrid_sleep_actually_sleeps() {
    let _ = mitos_power::sleep::hybrid_sleep::hybrid_sleep().await;
}
