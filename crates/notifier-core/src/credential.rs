//! Storage for the WaniKani personal access token.
//!
//! The token is a secret, so it is kept in the operating system's credential
//! store: the Keychain on macOS and the Credential Manager on Windows. On
//! platforms without one, a file with restricted permissions is used instead.

use std::path::{Path, PathBuf};

/// A place the API token can be kept.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// The credential store could not be reached.
    #[error("could not access the credential store: {0}")]
    Store(String),

    /// There is no token stored.
    #[error("no API token has been saved")]
    Missing,

    /// The token could not be read or written.
    #[error("could not read or write the stored API token: {0}")]
    Io(#[from] std::io::Error),
}

/// The service name the token is stored under.
const SERVICE: &str = "tanuki";

/// The account name the token is stored under.
const ACCOUNT: &str = "wanikani-api-token";

/// A store for the API token.
///
/// Implementations are shared between the polling task and the window, so
/// they have to be safe to use from several threads.
pub trait CredentialStore: Send + Sync {
    /// Returns the stored token, or [`CredentialError::Missing`] if there is none.
    fn token(&self) -> Result<String, CredentialError>;

    /// Stores the token, replacing any previous one.
    fn set_token(&self, token: &str) -> Result<(), CredentialError>;

    /// Removes the stored token, if any.
    fn clear_token(&self) -> Result<(), CredentialError>;
}

/// The store used on macOS and Windows, backed by the OS credential store.
#[derive(Debug, Clone, Default)]
pub struct SystemCredentialStore;

impl SystemCredentialStore {
    /// Creates the store.
    pub fn new() -> Self {
        Self
    }
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
impl CredentialStore for SystemCredentialStore {
    fn token(&self) -> Result<String, CredentialError> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)
            .map_err(|err| CredentialError::Store(err.to_string()))?;
        match entry.get_password() {
            Ok(token) => Ok(token),
            Err(keyring::Error::NoEntry) => Err(CredentialError::Missing),
            Err(err) => Err(CredentialError::Store(err.to_string())),
        }
    }

    fn set_token(&self, token: &str) -> Result<(), CredentialError> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)
            .map_err(|err| CredentialError::Store(err.to_string()))?;
        entry
            .set_password(token)
            .map_err(|err| CredentialError::Store(err.to_string()))
    }

    fn clear_token(&self) -> Result<(), CredentialError> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT)
            .map_err(|err| CredentialError::Store(err.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(CredentialError::Store(err.to_string())),
        }
    }
}

/// A fallback store that keeps the token in a file.
///
/// Used on platforms without an OS credential store. The file is created with
/// permissions only its owner can read.
#[derive(Debug, Clone)]
pub struct FileCredentialStore {
    path: PathBuf,
}

impl FileCredentialStore {
    /// Creates a store backed by the given file.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The file the token is kept in.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl CredentialStore for FileCredentialStore {
    fn token(&self) -> Result<String, CredentialError> {
        match std::fs::read_to_string(&self.path) {
            Ok(contents) => Ok(contents.trim().to_owned()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(CredentialError::Missing),
            Err(err) => Err(err.into()),
        }
    }

    fn set_token(&self, token: &str) -> Result<(), CredentialError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, token)?;
        restrict_permissions(&self.path)?;
        Ok(())
    }

    fn clear_token(&self) -> Result<(), CredentialError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

/// Makes a file readable and writable by its owner only.
#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), CredentialError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// Makes a file readable and writable by its owner only.
#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), CredentialError> {
    Ok(())
}

/// Picks the best credential store for the current platform.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn default_store(_fallback_dir: &Path) -> Box<dyn CredentialStore> {
    Box::new(SystemCredentialStore::new())
}

/// Picks the best credential store for the current platform.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn default_store(fallback_dir: &Path) -> Box<dyn CredentialStore> {
    Box::new(FileCredentialStore::new(fallback_dir.join("token")))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_store_round_trips_a_token() {
        let dir = std::env::temp_dir().join(format!("tanuki-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = FileCredentialStore::new(dir.join("token"));

        assert!(matches!(store.token(), Err(CredentialError::Missing)));

        store.set_token("secret-token").unwrap();
        assert_eq!(store.token().unwrap(), "secret-token");

        store.clear_token().unwrap();
        assert!(matches!(store.token(), Err(CredentialError::Missing)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clearing_a_missing_token_is_not_an_error() {
        let dir = std::env::temp_dir().join(format!("tanuki-test-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = FileCredentialStore::new(dir.join("token"));
        assert!(store.clear_token().is_ok());
    }
}
