use super::profile::ProfileSettings;

pub fn settings() -> ProfileSettings {
    ProfileSettings {
        cpu_governor: "schedutil".into(),
        cpu_boost: true,
        display_timeout_secs: 600,
        display_max_brightness_percent: 100,
        background_throttle: false,
        description: "Normal CPU scaling, normal display timeout, normal background policy".into(),
    }
}
