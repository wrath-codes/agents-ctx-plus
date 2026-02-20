use std::sync::{Arc, OnceLock};

use clerk_rs::ClerkConfiguration;
use clerk_rs::clerk::Clerk;
use clerk_rs::validators::authorizer::validate_jwt;
use clerk_rs::validators::jwks::MemoryCacheJwksProvider;

use crate::claims::ZenClaims;
use crate::error::AuthError;

/// Process-scoped JWKS provider cache.
/// Created on first use, reused for all subsequent validations.
/// The `MemoryCacheJwksProvider` internally caches public keys for 1 hour.
///
/// **Note**: This is bound to the `secret_key` from the first `validate()` call.
/// If a different secret key is passed later, the cached provider (and its key)
/// is still used. This is fine for the CLI (one key per process) but would need
/// rework for multi-tenant or long-running server use.
static JWKS_PROVIDER: OnceLock<Arc<MemoryCacheJwksProvider>> = OnceLock::new();

fn get_or_init_provider(secret_key: &str) -> Arc<MemoryCacheJwksProvider> {
    JWKS_PROVIDER
        .get_or_init(|| {
            let config = ClerkConfiguration::new(None, None, Some(secret_key.to_string()), None);
            let clerk = Clerk::new(config);
            Arc::new(MemoryCacheJwksProvider::new(clerk))
        })
        .clone()
}

/// Validate a Clerk JWT via JWKS and extract Zenith-specific claims.
///
/// Uses the Clerk Backend API (via `secret_key`) to fetch and cache JWKS
/// public keys. The provider is created once per process and reused.
///
/// # Errors
///
/// Returns `AuthError::JwksValidation` if the token is invalid, expired,
/// or the JWKS endpoint is unreachable.
pub async fn validate(jwt: &str, secret_key: &str) -> Result<ZenClaims, AuthError> {
    let provider = get_or_init_provider(secret_key);
    let clerk_jwt = validate_jwt(jwt, provider)
        .await
        .map_err(|e| AuthError::JwksValidation(e.to_string()))?;

    let expires_at = chrono::DateTime::from_timestamp(i64::from(clerk_jwt.exp), 0)
        .ok_or_else(|| AuthError::JwksValidation("invalid exp timestamp".into()))?;
    let org = clerk_jwt.org.as_ref();

    Ok(ZenClaims {
        raw_jwt: jwt.to_string(),
        user_id: clerk_jwt.sub.clone(),
        org_id: org.map(|o| o.id.clone()),
        org_slug: org.map(|o| o.slug.clone()),
        org_role: org.map(|o| o.role.clone()),
        expires_at,
    })
}
