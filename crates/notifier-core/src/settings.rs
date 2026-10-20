//! User-adjustable settings.

use serde::{Deserialize, Serialize};

/// Everything the user can configure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Whether to play a sound when reviews become available.
    pub sound: bool,

    /// Whether to show a system notification when reviews become available.
    pub notification: bool,

    /// Whether to launch Tanuki when the user logs in.
    pub launch_at_login: bool,

    /// Whether to also announce new lessons.
    pub notify_lessons: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound: true,
            notification: true,
            launch_at_login: false,
            notify_lessons: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let settings = Settings::default();
        assert!(settings.sound);
        assert!(settings.notification);
        assert!(!settings.launch_at_login);
        assert!(!settings.notify_lessons);
    }

    #[test]
    fn settings_round_trip_through_json() {
        let settings = Settings {
            sound: false,
            notification: true,
            launch_at_login: true,
            notify_lessons: true,
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: Settings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(settings, decoded);
    }

    #[test]
    fn missing_fields_fall_back_to_the_defaults() {
        let decoded: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(decoded, Settings::default());
    }
}
