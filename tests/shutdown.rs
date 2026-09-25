//! Shutdown/reboot/poweroff genuinely power off or restart the machine
//! running the test -- there is no safe way to exercise the real syscall
//! path in an automated test, so those are `#[ignore]`d; only run
//! deliberately, and only on a disposable VM you are fine losing.
//!
//! `logout()` IS safely tested unconditionally: it doesn't touch the
//! kernel at all today (mitos-session integration isn't implemented yet --
//! see audit.md), so it's currently a genuine no-op.

#[tokio::test]
async fn logout_is_currently_a_safe_no_op() {
    // Documents today's actual behavior (see src/shutdown/logout.rs). This
    // assertion will need revisiting once mitos-session integration lands.
    let result = mitos_power::shutdown::logout::logout("test").await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "actually powers off the test machine -- only run deliberately on a disposable VM"]
async fn poweroff_actually_powers_off() {
    let _ = mitos_power::shutdown::poweroff::poweroff("test").await;
}

#[tokio::test]
#[ignore = "actually reboots the test machine -- only run deliberately on a disposable VM"]
async fn reboot_actually_reboots() {
    let _ = mitos_power::shutdown::reboot::reboot("test", false).await;
}
