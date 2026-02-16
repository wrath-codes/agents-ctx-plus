//! Shared HTTP response helpers for registry clients.
//!
//! Centralizes status-code checks (429 rate limiting with `Retry-After`
//! parsing, non-success → [`RegistryError::Api`]) so individual registry
//! modules stay focused on request construction and response mapping.

use crate::error::RegistryError;

/// Check an HTTP response for common error conditions.
///
/// Returns the response unchanged on success. Handles:
/// - **429 Too Many Requests** → [`RegistryError::RateLimited`] with
///   `Retry-After` header parsing (falls back to 60 s if absent or
///   unparseable).
/// - **Non-success status** → [`RegistryError::Api`] with status code and
///   response body.
pub async fn check_response(
    resp: reqwest::Response,
) -> Result<reqwest::Response, RegistryError> {
    if resp.status() == 429 {
        let retry_after = parse_retry_after(&resp);
        return Err(RegistryError::RateLimited {
            retry_after_secs: retry_after,
        });
    }
    if !resp.status().is_success() {
        return Err(RegistryError::Api {
            status: resp.status().as_u16(),
            message: resp.text().await.unwrap_or_default(),
        });
    }
    Ok(resp)
}

/// Parse the `Retry-After` header as seconds, falling back to 60 s.
fn parse_retry_after(resp: &reqwest::Response) -> u64 {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_response(status: u16) -> reqwest::Response {
        reqwest::Response::from(
            ::http::Response::builder()
                .status(status)
                .body("")
                .unwrap(),
        )
    }

    fn mock_response_with_retry_after(status: u16, value: &str) -> reqwest::Response {
        reqwest::Response::from(
            ::http::Response::builder()
                .status(status)
                .header("Retry-After", value)
                .body("")
                .unwrap(),
        )
    }

    #[test]
    fn parse_retry_after_from_header() {
        let resp = mock_response_with_retry_after(429, "120");
        assert_eq!(parse_retry_after(&resp), 120);
    }

    #[test]
    fn parse_retry_after_missing_header() {
        let resp = mock_response(429);
        assert_eq!(parse_retry_after(&resp), 60);
    }

    #[test]
    fn parse_retry_after_non_numeric() {
        let resp = mock_response_with_retry_after(429, "not-a-number");
        assert_eq!(parse_retry_after(&resp), 60);
    }

    #[tokio::test]
    async fn check_response_rate_limited_with_header() {
        let resp = mock_response_with_retry_after(429, "30");
        let err = check_response(resp).await.unwrap_err();
        assert!(matches!(
            err,
            RegistryError::RateLimited {
                retry_after_secs: 30
            }
        ));
    }

    #[tokio::test]
    async fn check_response_rate_limited_default() {
        let resp = mock_response(429);
        let err = check_response(resp).await.unwrap_err();
        assert!(matches!(
            err,
            RegistryError::RateLimited {
                retry_after_secs: 60
            }
        ));
    }

    #[tokio::test]
    async fn check_response_api_error() {
        let resp = mock_response(500);
        let err = check_response(resp).await.unwrap_err();
        assert!(matches!(err, RegistryError::Api { status: 500, .. }));
    }

    #[tokio::test]
    async fn check_response_success() {
        let resp = mock_response(200);
        assert!(check_response(resp).await.is_ok());
    }
}
