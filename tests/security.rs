//! Permission-tier classification. The actual peer-credential enforcement
//! needs a real Unix socket connection with credentials attached, which is
//! exercised implicitly by tests/ipc.rs; this tests the pure
//! classification function that decides which tier a method needs.

use mitos_power::ipc::permissions::{required_privilege, Privilege};

#[test]
fn query_methods_are_public() {
    assert_eq!(required_privilege("GetBattery"), Privilege::Public);
    assert_eq!(required_privilege("Ping"), Privilege::Public);
    assert_eq!(required_privilege("ListInhibitors"), Privilege::Public);
}

#[test]
fn brightness_and_profile_changes_need_only_a_session() {
    assert_eq!(required_privilege("SetBrightness"), Privilege::Session);
    assert_eq!(required_privilege("SetProfile"), Privilege::Session);
}

#[test]
fn power_state_changes_are_privileged() {
    assert_eq!(required_privilege("Shutdown"), Privilege::Privileged);
    assert_eq!(required_privilege("Suspend"), Privilege::Privileged);
    assert_eq!(required_privilege("Reboot"), Privilege::Privileged);
}

#[test]
fn unknown_methods_default_to_the_strictest_tier() {
    assert_eq!(required_privilege("SomeMadeUpMethodName"), Privilege::Privileged);
}
