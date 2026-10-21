//! User-adjustable settings.

use crate::theme::Theme;
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

    /// The look and sound of the app.
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound: true,
            notification: true,
            // Tanuki is meant to be there when the user logs in, so this is
            // on by default.
            launch_at_login: true,
            notify_lessons: false,
            theme: Theme::default(),
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
        assert!(settings.launch_at_login);
        assert!(!settings.notify_lessons);
        assert_eq!(settings.theme, Theme::Automatic);
    }

    #[test]
    fn settings_round_trip_through_json() {
        let settings = Settings {
            sound: false,
            notification: true,
            launch_at_login: true,
            notify_lessons: true,
            theme: Theme::Halloween,
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

    #[test]
    fn settings_written_before_themes_existed_still_load() {
        // A settings file from an older version has no theme in it.
        let decoded: Settings = serde_json::from_str(
            r#"{"sound": true, "notification": true, "launch_at_login": true, "notify_lessons": false}"#,
        )
        .unwrap();
        assert_eq!(decoded.theme, Theme::Automatic);
    }
}
