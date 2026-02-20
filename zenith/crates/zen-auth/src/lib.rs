//! # zen-auth
//!
//! Clerk-based authentication for Zenith CLI.
//!
//! Provides browser login (`tiny_http` + `open`), JWKS JWT validation (`clerk-rs`),
//! OS keychain token storage (`keyring`), API key fallback for CI, and token
//! lifecycle management.

pub mod api_key;
pub mod browser_flow;
pub mod claims;
pub mod error;
pub mod jwks;
pub mod org;
pub mod refresh;
pub mod token_store;

pub use claims::ZenClaims;
pub use error::AuthError;

/// Resolve the best available auth token.
///
/// Priority: keyring → env var → file.
/// Does NOT validate the token (use [`resolve_and_validate`] for validation).
#[must_use]
pub fn resolve_token() -> Option<String> {
    token_store::load()
}

/// Full token resolution with JWKS validation.
///
/// Returns validated claims if a token exists and is valid + not near-expiry.
///
/// # Errors
///
/// Returns `AuthError` if JWKS validation encounters a network or parsing error.
pub async fn resolve_and_validate(secret_key: &str) -> Result<Option<ZenClaims>, AuthError> {
    refresh::check_stored_token(secret_key).await
}

/// Clear stored credentials.
///
/// # Errors
///
/// Returns `AuthError::TokenStoreError` if the credentials file cannot be removed.
pub fn logout() -> Result<(), AuthError> {
    token_store::delete()
}
