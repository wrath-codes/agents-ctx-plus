use crate::claims::ZenClaims;
use crate::error::AuthError;
use clerk_rs::ClerkConfiguration;
use clerk_rs::apis::clients_api;
use clerk_rs::clerk::Clerk;

/// Programmatic JWT generation via Clerk Backend API.
///
/// Creates a Clerk session and retrieves a JWT from the `zenith_cli` template.
/// For CI/headless environments where browser login is impossible.
///
/// # Security
///
/// The `secret_key` has full backend access. It is only used transiently to mint
/// a JWT — the secret key itself is never stored.
///
/// # Errors
///
/// Returns `AuthError::ApiKeyFailed` if session creation or JWT minting fails.
pub async fn login_with_api_key(
    secret_key: &str,
    user_id: &str,
) -> Result<ZenClaims, AuthError> {
    let client = reqwest::Client::new();

    // 1. Create session
    let session_resp = client
        .post("https://api.clerk.com/v1/sessions")
        .header("Authorization", format!("Bearer {secret_key}"))
        .json(&serde_json::json!({"user_id": user_id}))
        .send()
        .await
        .map_err(|e| AuthError::ApiKeyFailed(format!("create session: {e}")))?
        .error_for_status()
        .map_err(|e| AuthError::ApiKeyFailed(format!("create session: {e}")))?;
    let session = session_resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| AuthError::ApiKeyFailed(format!("parse session: {e}")))?;

    let session_id = session["id"]
        .as_str()
        .ok_or_else(|| AuthError::ApiKeyFailed("session response missing 'id'".into()))?;

    // 2. Get JWT from zenith_cli template
    let jwt = mint_token_for_session(secret_key, session_id).await?;

    let claims = crate::jwks::validate(&jwt, secret_key).await?;
    crate::token_store::store(&jwt)?;
    Ok(claims)
}

/// Mint a `zenith_cli` JWT for an existing Clerk session.
///
/// # Errors
///
/// Returns `AuthError::ApiKeyFailed` when token minting or parsing fails.
pub async fn mint_token_for_session(secret_key: &str, session_id: &str) -> Result<String, AuthError> {
    let client = reqwest::Client::new();
    let token_resp = client
        .post(format!(
            "https://api.clerk.com/v1/sessions/{session_id}/tokens/zenith_cli"
        ))
        .header("Authorization", format!("Bearer {secret_key}"))
        .send()
        .await
        .map_err(|e| AuthError::ApiKeyFailed(format!("get token: {e}")))?
        .error_for_status()
        .map_err(|e| AuthError::ApiKeyFailed(format!("get token: {e}")))?;
    let token = token_resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| AuthError::ApiKeyFailed(format!("parse token: {e}")))?;

    token["jwt"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| AuthError::ApiKeyFailed("token response missing 'jwt'".into()))
}

/// Resolve a Clerk session ID from a frontend/client token (e.g. `__clerk_db_jwt`).
///
/// # Errors
///
/// Returns `AuthError::ApiKeyFailed` when Clerk verification fails.
pub async fn resolve_session_id_from_client_token(
    secret_key: &str,
    client_token: &str,
) -> Result<Option<String>, AuthError> {
    let config = ClerkConfiguration::new(None, None, Some(secret_key.to_string()), None);
    let clerk = Clerk::new(config);

    let request = clerk_rs::models::VerifyClientRequest {
        token: Some(client_token.to_string()),
    };

    let client = clients_api::ClientApis::verify_client(&clerk, Some(request))
        .await
        .map_err(|error| AuthError::ApiKeyFailed(format!("verify client token: {error}")))?;

    if let Some(session_id) = client.last_active_session_id {
        return Ok(Some(session_id));
    }

    Ok(client.session_ids.into_iter().next())
}
