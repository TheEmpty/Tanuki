//! The look and sound of the app.
//!
//! A theme is more than a colour scheme: each one brings its own chime as
//! well, so a seasonal theme can ring like Halloween and sparkle like
//! Christmas without touching the rest of the app.
//!
//! The user picks a [`Theme`]. That is turned into the [`Look`] that is
//! actually in use, because the automatic theme follows the season: it is
//! Halloween in October and Christmas in December, and the plain look for
//! the rest of the year.
//!
//! What each one is called, and what it is called in a settings file, is in
//! [`THEMES`] and [`LOOKS`]. Those tables are the one place a theme or a look
//! is written down; the enums beside them are handles.

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// October, the month of Halloween.
const OCTOBER: u32 = 10;

/// December, the month of Christmas.
const DECEMBER: u32 = 12;

/// A theme: what the user has chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Follow the season, and the system's light or dark appearance.
    #[default]
    Automatic,
    /// Always follow the system's light or dark appearance, never a season.
    MatchSystem,
    /// Halloween, all year round.
    Halloween,
    /// Christmas, all year round.
    Christmas,
}

/// Everything that is known about a theme.
#[derive(Debug, Clone, Copy)]
pub struct ThemeInfo {
    /// The theme this describes.
    pub theme: Theme,
    /// A stable identifier, used in the settings file and by the interface.
    pub id: &'static str,
    /// The name shown in the interface.
    pub name: &'static str,
}

/// Every theme, in the order the interface offers them.
///
/// A theme that is not in here cannot be picked, so this is also the list the
/// dropdown shows.
pub const THEMES: &[ThemeInfo] = &[
    ThemeInfo {
        theme: Theme::Automatic,
        id: "automatic",
        name: "Automatic (Default)",
    },
    ThemeInfo {
        theme: Theme::MatchSystem,
        id: "match-system",
        name: "Match System",
    },
    ThemeInfo {
        theme: Theme::Halloween,
        id: "halloween",
        name: "Halloween",
    },
    ThemeInfo {
        theme: Theme::Christmas,
        id: "christmas",
        name: "Christmas",
    },
];

impl Theme {
    /// Everything that is known about this theme.
    pub fn info(self) -> &'static ThemeInfo {
        THEMES
            .iter()
            .find(|info| info.theme == self)
            .expect("every theme is described in THEMES")
    }

    /// A stable identifier, used in the settings file and by the interface.
    pub fn id(self) -> &'static str {
        self.info().id
    }

    /// The name shown in the interface.
    pub fn name(self) -> &'static str {
        self.info().name
    }

    /// Looks a theme up by its identifier.
    pub fn from_id(id: &str) -> Option<Theme> {
        THEMES
            .iter()
            .find(|info| info.id == id)
            .map(|info| info.theme)
    }

    /// The look this theme means on a given day.
    ///
    /// The automatic theme is Halloween in October and Christmas in
    /// December. The other themes are what they say, all year round.
    pub fn resolve(self, now: DateTime<Utc>) -> Look {
        match self {
            Theme::Automatic => match now.month() {
                OCTOBER => Look::Halloween,
                DECEMBER => Look::Christmas,
                _ => Look::Default,
            },
            Theme::MatchSystem => Look::Default,
            Theme::Halloween => Look::Halloween,
            Theme::Christmas => Look::Christmas,
        }
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

/// The look that is actually in use: the colours on screen and the chime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Look {
    /// The plain look, which follows the system's appearance.
    #[default]
    Default,
    /// The Halloween look.
    Halloween,
    /// The Christmas look.
    Christmas,
}

/// Everything that is known about a look.
#[derive(Debug, Clone, Copy)]
pub struct LookInfo {
    /// The look this describes.
    pub look: Look,
    /// A stable identifier, used by the interface to pick the colours.
    pub id: &'static str,
    /// The name of the look.
    pub name: &'static str,
}

/// Every look, in the order the interface offers them.
pub const LOOKS: &[LookInfo] = &[
    LookInfo {
        look: Look::Default,
        id: "default",
        name: "Default",
    },
    LookInfo {
        look: Look::Halloween,
        id: "halloween",
        name: "Halloween",
    },
    LookInfo {
        look: Look::Christmas,
        id: "christmas",
        name: "Christmas",
    },
];

impl Look {
    /// Everything that is known about this look.
    pub fn info(self) -> &'static LookInfo {
        LOOKS
            .iter()
            .find(|info| info.look == self)
            .expect("every look is described in LOOKS")
    }

    /// A stable identifier, used by the interface to pick the colours.
    pub fn id(self) -> &'static str {
        self.info().id
    }

    /// The name of the look.
    pub fn name(self) -> &'static str {
        self.info().name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// A date in the given month of 2024.
    fn in_month(month: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, month, 15, 12, 0, 0).unwrap()
    }

    #[test]
    fn automatic_is_the_default() {
        assert_eq!(Theme::default(), Theme::Automatic);
    }

    #[test]
    fn every_theme_is_described_and_offered_in_this_order() {
        // A theme that is missing from THEMES cannot be picked, so a new one
        // has to be added there too. This test is what says so.
        let ids: Vec<&str> = THEMES.iter().map(|info| info.id).collect();
        assert_eq!(
            ids,
            ["automatic", "match-system", "halloween", "christmas"],
            "a new theme needs a row in THEMES"
        );
    }

    #[test]
    fn every_look_is_described() {
        let ids: Vec<&str> = LOOKS.iter().map(|info| info.id).collect();
        assert_eq!(
            ids,
            ["default", "halloween", "christmas"],
            "a new look needs a row in LOOKS"
        );
    }

    #[test]
    fn a_theme_can_be_found_by_its_identifier() {
        for info in THEMES {
            assert_eq!(Theme::from_id(info.id), Some(info.theme));
        }
    }

    #[test]
    fn an_unknown_identifier_is_not_a_theme() {
        assert_eq!(Theme::from_id("launch-purple"), None);
    }

    #[test]
    fn the_names_are_the_ones_the_interface_shows() {
        assert_eq!(Theme::Automatic.name(), "Automatic (Default)");
        assert_eq!(Theme::MatchSystem.name(), "Match System");
        assert_eq!(Theme::Halloween.name(), "Halloween");
        assert_eq!(Theme::Christmas.name(), "Christmas");
    }

    #[test]
    fn themes_round_trip_through_json() {
        for info in THEMES {
            let encoded = serde_json::to_string(&info.theme).unwrap();
            let decoded: Theme = serde_json::from_str(&encoded).unwrap();
            assert_eq!(info.theme, decoded);
        }
    }

    #[test]
    fn looks_round_trip_through_json() {
        for info in LOOKS {
            let encoded = serde_json::to_string(&info.look).unwrap();
            let decoded: Look = serde_json::from_str(&encoded).unwrap();
            assert_eq!(info.look, decoded);
        }
    }

    #[test]
    fn the_identifiers_are_what_json_uses() {
        // The interface picks its colours with these, so a look's identifier
        // and the name it has in a settings file have to be the same thing.
        for info in THEMES {
            assert_eq!(
                serde_json::to_string(&info.theme).unwrap(),
                format!("\"{}\"", info.id)
            );
        }
        for info in LOOKS {
            assert_eq!(
                serde_json::to_string(&info.look).unwrap(),
                format!("\"{}\"", info.id)
            );
        }
    }

    #[test]
    fn a_theme_this_version_does_not_know_falls_back_to_the_default() {
        // A settings file written by a newer version must not break the app.
        let decoded: Theme = serde_json::from_str("\"spring\"").unwrap();
        assert_eq!(decoded, Theme::Automatic);
    }

    #[test]
    fn the_automatic_theme_is_halloween_in_october() {
        let theme = Theme::Automatic;
        assert_eq!(theme.resolve(in_month(10)), Look::Halloween);
    }

    #[test]
    fn the_automatic_theme_is_christmas_in_december() {
        let theme = Theme::Automatic;
        assert_eq!(theme.resolve(in_month(12)), Look::Christmas);
    }

    #[test]
    fn the_automatic_theme_is_plain_for_the_rest_of_the_year() {
        let theme = Theme::Automatic;
        for month in [1, 2, 3, 4, 5, 6, 7, 8, 9, 11] {
            assert_eq!(
                theme.resolve(in_month(month)),
                Look::Default,
                "month {month} should be plain"
            );
        }
    }

    #[test]
    fn the_seasonal_themes_do_not_change_with_the_month() {
        for month in 1..=12 {
            assert_eq!(Theme::Halloween.resolve(in_month(month)), Look::Halloween);
            assert_eq!(Theme::Christmas.resolve(in_month(month)), Look::Christmas);
        }
    }

    #[test]
    fn the_match_system_theme_never_turns_into_a_season() {
        for month in 1..=12 {
            assert_eq!(
                Theme::MatchSystem.resolve(in_month(month)),
                Look::Default,
                "month {month} should stay plain"
            );
        }
    }
}
