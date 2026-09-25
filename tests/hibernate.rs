//! See tests/suspend.rs for why hibernate can't be safely exercised in an
//! automated test that might run on real hardware.

#[tokio::test]
#[ignore = "hibernates the machine running the test if hibernate is actually supported -- only run deliberately on a disposable VM"]
async fn hibernate_round_trips() {
    let _ = mitos_power::sleep::hibernate::hibernate().await;
}

#[tokio::test]
#[ignore = "suspends now and may hibernate later on a real machine -- only run deliberately on a disposable VM"]
async fn suspend_then_hibernate_round_trips() {
    let _ = mitos_power::sleep::suspend_then_hibernate::suspend_then_hibernate(1).await;
}
