//! Shared state and the on-disk settings file.

use notifier_core::credential::{CredentialError, CredentialStore};
use notifier_core::detect::Detector;
use notifier_core::settings::Settings;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::menu::MenuItem;
use tauri::Wry;

/// Everything the app keeps around while it runs.
pub struct AppState {
    /// The user's settings.
    pub settings: Mutex<Settings>,
    /// The detector, which decides when reviews count as "new".
    pub detector: Mutex<Detector>,
    /// The menu bar item that shows the current status.
    pub status_item: MenuItem<Wry>,
    /// Where the API token is kept.
    pub credentials: Box<dyn CredentialStore>,
    /// The API token, read from the credential store once and kept in memory.
    ///
    /// Reading the credential store can ask the operating system for
    /// permission, so it is only done when the token actually changes.
    pub token: Mutex<Option<String>>,
    /// Where the settings file lives.
    pub settings_path: PathBuf,
    /// Where the detector state is kept.
    pub state_path: PathBuf,
}

impl AppState {
    /// Creates the state, loading the settings and detector from disk.
    pub fn load(
        config_dir: &Path,
        credentials: Box<dyn CredentialStore>,
        status_item: MenuItem<Wry>,
    ) -> Result<Self, SettingsError> {
        let settings_path = config_dir.join("settings.json");
        let state_path = config_dir.join("state.json");

        let settings = read_json(&settings_path)?.unwrap_or_default();
        let detector = read_json(&state_path)?.unwrap_or_default();

        Ok(Self {
            settings: Mutex::new(settings),
            detector: Mutex::new(detector),
            status_item,
            credentials,
            token: Mutex::new(None),
            settings_path,
            state_path,
        })
    }

    /// The API token, if one has been saved.
    ///
    /// The credential store is only consulted the first time, so a system
    /// that asks for permission does not ask on every poll.
    pub fn token(&self) -> Result<Option<String>, CredentialError> {
        let mut cached = self.token.lock().expect("the token lock should not be poisoned");

        if let Some(token) = cached.as_ref() {
            return Ok(Some(token.clone()));
        }

        match self.credentials.token() {
            Ok(token) => {
                *cached = Some(token.clone());
                Ok(Some(token))
            }
            Err(CredentialError::Missing) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Stores a new API token and forgets the cached one.
    pub fn set_token(&self, token: &str) -> Result<(), CredentialError> {
        self.credentials.set_token(token)?;
        *self.token.lock().expect("the token lock should not be poisoned") = Some(token.to_owned());
        Ok(())
    }

    /// Removes the stored API token.
    pub fn clear_token(&self) -> Result<(), CredentialError> {
        self.credentials.clear_token()?;
        *self.token.lock().expect("the token lock should not be poisoned") = None;
        Ok(())
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

    /// Saves the detector to disk.
    pub fn save_detector(&self) -> Result<(), SettingsError> {
        let detector = *self
            .detector
            .lock()
            .map_err(|_| SettingsError::Poisoned)?;
        write_json(&self.state_path, &detector)
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
