//! The look and sound of the app.
//!
//! A theme is more than a colour scheme: each one brings its own chime as
//! well, so a seasonal theme can ring like Halloween and sparkle like
//! Christmas without touching the rest of the app.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A theme: how Tanuki looks and sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Follow the system's light or dark appearance.
    #[default]
    Automatic,
    /// The purple theme Tanuki launched with.
    LaunchPurple,
}

impl Theme {
    /// Every theme, in the order they should be offered.
    pub const ALL: [Theme; 2] = [Theme::Automatic, Theme::LaunchPurple];

    /// A stable identifier, used in the settings file and by the interface.
    pub fn id(self) -> &'static str {
        match self {
            Theme::Automatic => "automatic",
            Theme::LaunchPurple => "launch-purple",
        }
    }

    /// The name shown in the interface.
    pub fn name(self) -> &'static str {
        match self {
            Theme::Automatic => "Automatic (Default)",
            Theme::LaunchPurple => "Launch Purple",
        }
    }

    /// Looks a theme up by its identifier.
    pub fn from_id(id: &str) -> Option<Theme> {
        Theme::ALL.iter().copied().find(|theme| theme.id() == id)
    }
}

impl Serialize for Theme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        // A theme this version does not know about falls back to the default
        // rather than making the whole settings file unreadable.
        Ok(Theme::from_id(&id).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_is_the_default() {
        assert_eq!(Theme::default(), Theme::Automatic);
    }

    #[test]
    fn the_identifiers_are_unique() {
        let mut ids: Vec<&str> = Theme::ALL.iter().map(|theme| theme.id()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "every theme needs its own identifier");
    }

    #[test]
    fn a_theme_can_be_found_by_its_identifier() {
        for theme in Theme::ALL {
            assert_eq!(Theme::from_id(theme.id()), Some(theme));
        }
    }

    #[test]
    fn an_unknown_identifier_is_not_a_theme() {
        assert_eq!(Theme::from_id("halloween"), None);
    }

    #[test]
    fn the_names_are_the_ones_the_interface_shows() {
        assert_eq!(Theme::Automatic.name(), "Automatic (Default)");
        assert_eq!(Theme::LaunchPurple.name(), "Launch Purple");
    }

    #[test]
    fn themes_round_trip_through_json() {
        for theme in Theme::ALL {
            let encoded = serde_json::to_string(&theme).unwrap();
            let decoded: Theme = serde_json::from_str(&encoded).unwrap();
            assert_eq!(theme, decoded);
        }
    }

    #[test]
    fn the_identifiers_are_what_json_uses() {
        let encoded = serde_json::to_string(&Theme::LaunchPurple).unwrap();
        assert_eq!(encoded, "\"launch-purple\"");
    }

    #[test]
    fn a_theme_this_version_does_not_know_falls_back_to_the_default() {
        // A settings file written by a newer version must not break the app.
        let decoded: Theme = serde_json::from_str("\"halloween\"").unwrap();
        assert_eq!(decoded, Theme::Automatic);
    }
}
