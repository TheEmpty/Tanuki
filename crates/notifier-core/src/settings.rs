//! User-adjustable settings.

use crate::theme::{Look, Theme};
use chrono::{DateTime, Utc};
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

    /// Whether the theme is allowed to pick the chime.
    ///
    /// When this is off, the standard bell is played whatever the theme is.
    pub themed_sounds: bool,

    /// Whether Command-Q quits the app rather than putting the window away.
    pub quit_with_command_q: bool,
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
            // The themes are the point of the themes, so they get to pick the
            // chime unless the user says otherwise.
            themed_sounds: true,
            // Tanuki is meant to stay out of the way, so Command-Q only puts
            // the window away unless the user asks for it to quit.
            quit_with_command_q: false,
        }
    }
}

impl Settings {
    /// The look whose chime should be played.
    ///
    /// The user can turn the themed sounds off, in which case the standard
    /// bell is played whatever the theme is.
    pub fn sound_look(&self, now: DateTime<Utc>) -> Look {
        if self.themed_sounds {
            self.theme.resolve(now)
        } else {
            Look::Default
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
        assert!(settings.themed_sounds);
        assert!(!settings.quit_with_command_q);
    }

    #[test]
    fn settings_round_trip_through_json() {
        let settings = Settings {
            sound: false,
            notification: true,
            launch_at_login: true,
            notify_lessons: true,
            theme: Theme::Halloween,
            themed_sounds: false,
            quit_with_command_q: true,
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
        assert!(decoded.themed_sounds);
    }

    #[test]
    fn the_theme_picks_the_chime_when_it_is_allowed_to() {
        let now = Utc::now();
        let settings = Settings {
            theme: Theme::Halloween,
            themed_sounds: true,
            ..Settings::default()
        };
        assert_eq!(settings.sound_look(now), Look::Halloween);
    }

    #[test]
    fn the_standard_bell_is_played_when_the_themes_are_not_allowed_to() {
        let now = Utc::now();
        let settings = Settings {
            theme: Theme::Halloween,
            themed_sounds: false,
            ..Settings::default()
        };
        assert_eq!(settings.sound_look(now), Look::Default);
    }
}
