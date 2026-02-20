use base64::Engine as _;

use crate::claims::ZenClaims;
use crate::error::AuthError;

const EXPIRY_BUFFER_SECS: i64 = 60;

/// Check if a stored token is still valid.
///
/// Returns `Some(claims)` if the token is valid and not near-expiry.
/// Returns `None` if no token is stored, or the token is expired/near-expiry.
///
/// # Errors
///
/// Returns `AuthError` if JWKS validation encounters a network or parsing error
/// (distinct from an expired token, which returns `Ok(None)`).
pub async fn check_stored_token(secret_key: &str) -> Result<Option<ZenClaims>, AuthError> {
    let Some(jwt) = crate::token_store::load() else {
        return Ok(None);
    };

    let claims = crate::jwks::validate(&jwt, secret_key).await?;
    if claims.is_near_expiry(EXPIRY_BUFFER_SECS) {
        tracing::warn!(
            expires_at = %claims.expires_at,
            "auth token expires within {EXPIRY_BUFFER_SECS}s — re-authenticate with `znt auth login`",
        );
        return Ok(None);
    }

    Ok(Some(claims))
}

/// Decode JWT `exp` claim without full JWKS validation (for quick expiry checks).
///
/// This is a best-effort check — it does NOT verify the JWT signature.
/// Use `check_stored_token()` for full validation.
///
/// # Errors
///
/// Returns `AuthError::Other` if the JWT format is invalid or the `exp` claim
/// is missing or cannot be parsed.
pub fn decode_expiry(jwt: &str) -> Result<chrono::DateTime<chrono::Utc>, AuthError> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::Other("invalid JWT format".into()));
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| AuthError::Other(format!("base64 decode failed: {e}")))?;
    let value: serde_json::Value = serde_json::from_slice(&payload)
        .map_err(|e| AuthError::Other(format!("JSON parse failed: {e}")))?;
    let exp = value["exp"]
        .as_i64()
        .ok_or_else(|| AuthError::Other("missing exp claim".into()))?;
    chrono::DateTime::from_timestamp(exp, 0)
        .ok_or_else(|| AuthError::Other("invalid exp timestamp".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_jwt_with_exp(exp: i64) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(format!(r#"{{"sub":"user_123","exp":{exp}}}"#));
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("fake_sig");
        format!("{header}.{payload}.{signature}")
    }

    #[test]
    fn decode_expiry_valid_jwt() {
        let future_exp = chrono::Utc::now().timestamp() + 3600;
        let jwt = make_jwt_with_exp(future_exp);
        let result = decode_expiry(&jwt);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.timestamp(), future_exp);
    }

    #[test]
    fn decode_expiry_expired_jwt() {
        let past_exp = chrono::Utc::now().timestamp() - 3600;
        let jwt = make_jwt_with_exp(past_exp);
        let result = decode_expiry(&jwt);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert!(dt < chrono::Utc::now());
    }

    #[test]
    fn decode_expiry_invalid_format() {
        let result = decode_expiry("not-a-jwt");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid JWT format")
        );
    }

    #[test]
    fn decode_expiry_missing_exp_claim() {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#);
        let payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"sub":"user_123"}"#);
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("fake_sig");
        let jwt = format!("{header}.{payload}.{signature}");

        let result = decode_expiry(&jwt);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("missing exp claim")
        );
    }

    #[test]
    fn decode_expiry_bad_base64() {
        let result = decode_expiry("header.!!!invalid!!!.signature");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("base64 decode failed")
        );
    }
}
