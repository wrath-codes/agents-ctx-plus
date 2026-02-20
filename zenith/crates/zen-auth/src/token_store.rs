use std::fs;
use std::path::PathBuf;

use crate::error::AuthError;

const DEFAULT_KEYRING_SERVICE: &str = "zenith-cli";
const KEYRING_USER: &str = "clerk-jwt";
const CREDENTIALS_FILE_NAME: &str = "credentials";

/// Returns the keyring service name.
///
/// Defaults to `"zenith-cli"`. Override via `ZENITH_KEYRING_SERVICE` env var
/// for testing (e.g., `"zenith-cli-test"`) to avoid touching production credentials.
fn keyring_service() -> String {
    std::env::var("ZENITH_KEYRING_SERVICE").unwrap_or_else(|_| DEFAULT_KEYRING_SERVICE.to_string())
}

/// Store a JWT in the OS keychain. Falls back to file if keyring unavailable.
///
/// # Errors
///
/// Returns `AuthError::TokenStoreError` if both keyring and file storage fail.
pub fn store(jwt: &str) -> Result<(), AuthError> {
    match keyring::Entry::new(&keyring_service(), KEYRING_USER) {
        Ok(entry) => match entry.set_password(jwt) {
            Ok(()) => Ok(()),
            Err(error) => {
                tracing::warn!(%error, "keyring store failed; falling back to file");
                store_file(jwt)
            }
        },
        Err(error) => {
            tracing::warn!(%error, "keyring unavailable; falling back to file");
            store_file(jwt)
        }
    }
}

/// Load a JWT. Priority: keyring → `ZENITH_AUTH__TOKEN` env → file (`~/.zenith/credentials`).
#[must_use]
pub fn load() -> Option<String> {
    // 1. Keyring
    if let Ok(entry) = keyring::Entry::new(&keyring_service(), KEYRING_USER)
        && let Ok(token) = entry.get_password()
        && !token.is_empty()
    {
        return Some(token);
    }

    // 2. Environment variable
    if let Ok(token) = std::env::var("ZENITH_AUTH__TOKEN") {
        if !token.is_empty() {
            return Some(token);
        }
    }

    // 3. File fallback
    load_file()
}

/// Delete stored credentials from keyring and file.
///
/// # Errors
///
/// Returns `AuthError::TokenStoreError` if the credentials file cannot be removed.
pub fn delete() -> Result<(), AuthError> {
    // Delete from keyring (ignore errors — may not exist)
    if let Ok(entry) = keyring::Entry::new(&keyring_service(), KEYRING_USER) {
        let _ = entry.delete_credential();
    }

    // Delete credentials file
    let path = credentials_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| {
            AuthError::TokenStoreError(format!("failed to delete {}: {e}", path.display()))
        })?;
    }

    Ok(())
}

/// Detect which tier the current token came from (for status display).
#[must_use]
pub fn detect_token_source() -> Option<String> {
    if let Ok(entry) = keyring::Entry::new(&keyring_service(), KEYRING_USER)
        && entry.get_password().is_ok_and(|t| !t.is_empty())
    {
        return Some("keyring".into());
    }
    if std::env::var("ZENITH_AUTH__TOKEN").is_ok_and(|t| !t.is_empty()) {
        return Some("env".into());
    }
    if load_file().is_some() {
        return Some("file".into());
    }
    None
}

// --- Private file helpers ---

fn credentials_path() -> Result<PathBuf, AuthError> {
    dirs::home_dir()
        .map(|h| h.join(".zenith").join(CREDENTIALS_FILE_NAME))
        .ok_or_else(|| {
            AuthError::TokenStoreError("home directory not found — cannot store credentials".into())
        })
}

fn store_file(jwt: &str) -> Result<(), AuthError> {
    let path = credentials_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AuthError::TokenStoreError(format!("mkdir {}: {e}", parent.display())))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = fs::set_permissions(parent, fs::Permissions::from_mode(0o700)) {
                tracing::warn!("failed to chmod 0700 {}: {e}", parent.display());
            }
        }
    }
    fs::write(&path, jwt)
        .map_err(|e| AuthError::TokenStoreError(format!("write {}: {e}", path.display())))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| AuthError::TokenStoreError(format!("chmod {}: {e}", path.display())))?;
    }

    Ok(())
}

fn load_file() -> Option<String> {
    let path = credentials_path().ok()?;
    fs::read_to_string(&path)
        .ok()
        .filter(|s| !s.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_path_is_under_home() {
        let path = credentials_path().expect("should resolve");
        assert!(path.ends_with(".zenith/credentials"));
    }

    #[test]
    fn file_store_load_delete_cycle() {
        let tmp = tempfile::TempDir::new().expect("tmp dir");
        let creds_path = tmp.path().join("credentials");

        // Store
        std::fs::write(&creds_path, "test_jwt_abc123").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&creds_path, std::fs::Permissions::from_mode(0o600))
                .expect("chmod");
        }

        // Load
        let content = std::fs::read_to_string(&creds_path).expect("read");
        assert_eq!(content, "test_jwt_abc123");

        // Verify permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&creds_path)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600, "credentials file should be 0600");
        }

        // Delete
        std::fs::remove_file(&creds_path).expect("delete");
        assert!(!creds_path.exists());
    }

    #[test]
    fn load_file_ignores_empty_content() {
        let tmp = tempfile::TempDir::new().expect("tmp dir");
        let creds_path = tmp.path().join("credentials");

        std::fs::write(&creds_path, "   \n  ").expect("write");
        let content = std::fs::read_to_string(&creds_path)
            .ok()
            .filter(|s| !s.trim().is_empty());
        assert!(content.is_none(), "whitespace-only should return None");
    }
}
