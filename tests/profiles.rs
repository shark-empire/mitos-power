//! End-to-end profile switching through the public API.

use mitos_power::profiles::ProfileManager;

#[test]
fn built_in_profiles_are_always_available_and_switchable() {
    let mut manager = ProfileManager::new("balanced", vec![]).expect("balanced is always defined");
    assert_eq!(manager.current().name, "balanced");

    for name in ["performance", "balanced", "powersave"] {
        let info = manager.set(name).expect("built-in profiles must always be settable");
        assert_eq!(info.name, name);
    }

    assert!(manager.set("does-not-exist").is_err());
}

#[test]
fn unknown_default_profile_is_rejected_at_construction() {
    assert!(ProfileManager::new("not-a-real-profile", vec![]).is_err());
}
