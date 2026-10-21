//! Storage for the WaniKani personal access token.
//!
//! The token is kept in the application's own configuration directory, in a
//! file only its owner can read.
//!
//! The system's credential store would be the more usual home for a secret,
//! but on macOS the keychain ties an item to the application that created
//! it. As soon as the application is rebuilt or upgraded, that tie no longer
//! holds and macOS asks the user for their password before the token can be
//! read again — and if the prompt is dismissed rather than always allowed,
//! the token has to be typed in from scratch. A read-only API token does not
//! need that ceremony, so it lives in a file next to the rest of the
//! settings instead.

use std::path::{Path, PathBuf};

/// The name of the file the token is kept in, within the configuration
/// directory.
pub const TOKEN_FILE: &str = "token";

/// Something went wrong with the stored token.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// There is no token stored.
    #[error("no API token has been saved")]
    Missing,

    /// The token could not be read or written.
    #[error("could not read or write the stored API token: {0}")]
    Io(#[from] std::io::Error),
}

/// The store for the API token.
///
/// It is shared between the polling task and the window, so it has to be
/// safe to use from several threads.
#[derive(Debug, Clone)]
pub struct TokenStore {
    path: PathBuf,
}

impl TokenStore {
    /// Creates a store that keeps the token in the given file.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Creates a store that keeps the token in the given directory.
    pub fn in_dir(dir: impl AsRef<Path>) -> Self {
        Self::new(dir.as_ref().join(TOKEN_FILE))
    }

    /// The file the token is kept in.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the stored token, or [`CredentialError::Missing`] if there is
    /// none.
    pub fn token(&self) -> Result<String, CredentialError> {
        match std::fs::read_to_string(&self.path) {
            Ok(contents) => Ok(contents.trim().to_owned()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(CredentialError::Missing),
            Err(err) => Err(err.into()),
        }
    }

    /// Stores the token, replacing any previous one.
    pub fn set_token(&self, token: &str) -> Result<(), CredentialError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_private(&self.path, token)?;
        Ok(())
    }

    /// Removes the stored token, if any.
    pub fn clear_token(&self) -> Result<(), CredentialError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

/// Writes a file that only its owner can read.
///
/// The permissions are set as the file is created, so the token is never on
/// disk in a state where other users can read it.
#[cfg(unix)]
fn write_private(path: &Path, contents: &str) -> Result<(), CredentialError> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;

    file.write_all(contents.as_bytes())?;

    // The mode above only applies to a file that did not exist yet.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// Writes a file that only its owner can read.
#[cfg(not(unix))]
fn write_private(path: &Path, contents: &str) -> Result<(), CredentialError> {
    std::fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tanuki-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_store_round_trips_a_token() {
        let dir = scratch("round-trip");
        let store = TokenStore::in_dir(&dir);

        assert!(matches!(store.token(), Err(CredentialError::Missing)));

        store.set_token("secret-token").unwrap();
        assert_eq!(store.token().unwrap(), "secret-token");

        store.clear_token().unwrap();
        assert!(matches!(store.token(), Err(CredentialError::Missing)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clearing_a_missing_token_is_not_an_error() {
        let dir = scratch("empty");
        let store = TokenStore::in_dir(&dir);
        assert!(store.clear_token().is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_token_is_kept_in_a_file_of_its_own() {
        let dir = scratch("file");
        let store = TokenStore::in_dir(&dir);
        assert_eq!(store.path(), dir.join(TOKEN_FILE));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn the_token_file_is_not_readable_by_others() {
        use std::os::unix::fs::PermissionsExt;

        let dir = scratch("perms");
        let store = TokenStore::in_dir(&dir);
        store.set_token("secret-token").unwrap();

        let mode = std::fs::metadata(store.path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "the token must not be world readable");
        assert_eq!(mode & 0o600, 0o600, "the owner must be able to read it");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
