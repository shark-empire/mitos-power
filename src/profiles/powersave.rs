use super::profile::ProfileSettings;

pub fn settings() -> ProfileSettings {
    ProfileSettings {
        cpu_governor: "powersave".into(),
        cpu_boost: false,
        display_timeout_secs: 180,
        display_max_brightness_percent: 60,
        background_throttle: true,
        description: "Reduced CPU frequency, reduced brightness, shorter idle timeout, aggressive power saving".into(),
    }
}
