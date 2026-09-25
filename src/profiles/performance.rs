use super::profile::ProfileSettings;

pub fn settings() -> ProfileSettings {
    ProfileSettings {
        cpu_governor: "performance".into(),
        cpu_boost: true,
        display_timeout_secs: 900,
        display_max_brightness_percent: 100,
        background_throttle: false,
        description: "CPU boost on, high responsiveness, reduced power-saving".into(),
    }
}
