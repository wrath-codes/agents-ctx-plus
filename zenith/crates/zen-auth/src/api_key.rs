use crate::claims::ZenClaims;
use crate::error::AuthError;

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

    let jwt = token["jwt"]
        .as_str()
        .ok_or_else(|| AuthError::ApiKeyFailed("token response missing 'jwt'".into()))?;

    let claims = crate::jwks::validate(jwt, secret_key).await?;
    crate::token_store::store(jwt)?;
    Ok(claims)
}
