//! Shared state and the on-disk settings file.

use notifier_core::detect::Detector;
use notifier_core::level::LevelWatcher;
use notifier_core::settings::Settings;
use notifier_core::theme::Look;
use notifier_core::token::{TokenError, TokenStore};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::menu::MenuItem;
use tauri::Wry;

/// What the app last found out about the review queue.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Status {
    /// Whether an API token has been saved.
    pub configured: bool,
    /// How many reviews are waiting.
    pub reviews: u32,
    /// When the next batch of reviews unlocks.
    pub next_review_at: Option<String>,
    /// When the last check happened.
    pub last_checked: Option<String>,
    /// The last error, if a check failed.
    pub error: Option<String>,
    /// The look that is in use, so the window keeps up with the season.
    pub look: Look,
}

/// Everything the app keeps around while it runs.
pub struct AppState {
    /// The user's settings.
    pub settings: Mutex<Settings>,
    /// The detector, which decides when reviews count as "new".
    pub detector: Mutex<Detector>,
    /// The menu bar item that shows the current status.
    pub status_item: MenuItem<Wry>,
    /// Where the API token is kept.
    pub tokens: TokenStore,
    /// The API token, read from the file once and kept in memory.
    ///
    /// The file is only read the first time, so it is not read again on
    /// every poll.
    pub token: Mutex<Option<String>>,
    /// The level watcher, which notices when the user has levelled up.
    pub level: Mutex<LevelWatcher>,
    /// Whether the user has been told that Command-Q does not quit.
    pub told_about_command_q: Mutex<bool>,
    /// The last thing the app found out about the review queue.
    ///
    /// The window asks for this when it opens, so it never has to wait for
    /// the next poll to show something sensible.
    pub status: Mutex<Status>,
    /// Where the settings file lives.
    pub settings_path: PathBuf,
    /// Where the state the app remembers lives.
    pub state_path: PathBuf,
}

/// What the app remembers between runs.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct Remembered {
    /// The level the user was last seen at.
    level: Option<u32>,
    /// Whether the user has been told that Command-Q does not quit.
    told_about_command_q: bool,
}

impl AppState {
    /// Creates the state, loading the settings from disk.
    pub fn load(
        config_dir: &Path,
        tokens: TokenStore,
        status_item: MenuItem<Wry>,
    ) -> Result<Self, SettingsError> {
        let settings_path = config_dir.join("settings.json");
        let state_path = config_dir.join("state.json");

        let settings = read_json(&settings_path)?.unwrap_or_default();
        let remembered: Remembered = read_json(&state_path)?.unwrap_or_default();

        Ok(Self {
            settings: Mutex::new(settings),
            // The detector starts fresh on every launch, so reviews that are
            // already waiting are announced when the app starts.
            detector: Mutex::new(Detector::new()),
            status_item,
            tokens,
            token: Mutex::new(None),
            // The level the user was last seen at is remembered, so a level
            // up that happened while the app was not running is still
            // noticed.
            level: Mutex::new(match remembered.level {
                Some(level) => LevelWatcher::seen_at(level),
                None => LevelWatcher::new(),
            }),
            told_about_command_q: Mutex::new(remembered.told_about_command_q),
            status: Mutex::new(Status::default()),
            settings_path,
            state_path,
        })
    }

    /// The API token, if one has been saved.
    ///
    /// The file is only read the first time, so it is not read again on
    /// every poll.
    pub fn token(&self) -> Result<Option<String>, TokenError> {
        let mut cached = self
            .token
            .lock()
            .expect("the token lock should not be poisoned");

        if let Some(token) = cached.as_ref() {
            return Ok(Some(token.clone()));
        }

        match self.tokens.token() {
            Ok(token) => {
                *cached = Some(token.clone());
                Ok(Some(token))
            }
            Err(TokenError::Missing) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Stores a new API token and forgets the cached one.
    pub fn set_token(&self, token: &str) -> Result<(), TokenError> {
        self.tokens.set_token(token)?;
        *self
            .token
            .lock()
            .expect("the token lock should not be poisoned") = Some(token.to_owned());
        Ok(())
    }

    /// Removes the stored API token.
    pub fn clear_token(&self) -> Result<(), TokenError> {
        self.tokens.clear_token()?;
        *self
            .token
            .lock()
            .expect("the token lock should not be poisoned") = None;
        Ok(())
    }

    /// The last thing the app found out about the review queue.
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .expect("the status lock should not be poisoned")
            .clone()
    }

    /// Records what the app found out about the review queue.
    pub fn set_status(&self, status: Status) {
        *self
            .status
            .lock()
            .expect("the status lock should not be poisoned") = status;
    }

    /// Feeds in the user's level and reports whether it has gone up.
    ///
    /// The level is remembered, so a level up that happened while the app
    /// was not running is noticed too.
    pub fn observe_level(&self, level: u32) -> bool {
        let (went_up, changed) = {
            let mut watcher = self
                .level
                .lock()
                .expect("the level lock should not be poisoned");

            let before = watcher.seen();
            let went_up = watcher.observe(level);
            (went_up, watcher.seen() != before)
        };

        if changed {
            self.save_remembered();
        }

        went_up
    }

    /// Whether to tell the user that Tanuki is still running.
    ///
    /// It is only worth saying once, so the app does not nag.
    ///
    /// Only macOS has a Command-Q to warn about.
    #[cfg(target_os = "macos")]
    pub fn should_warn_about_command_q(&self) -> bool {
        !*self
            .told_about_command_q
            .lock()
            .expect("the lock should not be poisoned")
    }

    /// Remembers that the user has been told Tanuki is still running.
    #[cfg(target_os = "macos")]
    pub fn remember_command_q_warning(&self) {
        {
            let mut told = self
                .told_about_command_q
                .lock()
                .expect("the lock should not be poisoned");
            if *told {
                return;
            }
            *told = true;
        }

        self.save_remembered();
    }

    /// Writes down what the app remembers between runs.
    fn save_remembered(&self) {
        let remembered = Remembered {
            level: self
                .level
                .lock()
                .expect("the level lock should not be poisoned")
                .seen(),
            told_about_command_q: *self
                .told_about_command_q
                .lock()
                .expect("the lock should not be poisoned"),
        };

        if let Err(error) = write_json(&self.state_path, &remembered) {
            eprintln!("tanuki: could not remember: {error}");
        }
    }

    /// Saves the settings to disk.
    pub fn save_settings(&self) -> Result<(), SettingsError> {
        let settings = self
            .settings
            .lock()
            .map_err(|_| SettingsError::Poisoned)?
            .clone();
        write_json(&self.settings_path, &settings)
    }
}

/// Something went wrong while reading or writing the settings.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// The file could not be read or written.
    #[error("could not read or write the settings: {0}")]
    Io(#[from] std::io::Error),

    /// The file could not be parsed.
    #[error("could not parse the settings: {0}")]
    Parse(#[from] serde_json::Error),

    /// A lock was poisoned by a panic elsewhere.
    #[error("the app state is in an inconsistent state")]
    Poisoned,
}

impl serde::Serialize for SettingsError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// Reads a JSON file, returning `None` when it does not exist.
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>, SettingsError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Writes a JSON file, creating the parent directory if needed.
fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), SettingsError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    std::fs::write(path, bytes)?;
    Ok(())
}
