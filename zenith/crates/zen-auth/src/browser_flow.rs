use crate::claims::ZenClaims;
use crate::error::AuthError;

/// Execute the browser-based Clerk login flow.
///
/// 1. Start `tiny_http` on localhost (random port)
/// 2. Open browser to Clerk sign-in with redirect to localhost
/// 3. Wait for callback with JWT (in `spawn_blocking` — `tiny_http::recv` blocks)
/// 4. Validate JWT via JWKS
/// 5. Store in keyring
///
/// # Errors
///
/// Returns `AuthError::BrowserFlowFailed` if the server cannot bind, the browser
/// cannot open, or the callback times out.
pub async fn login(
    clerk_frontend_api: &str,
    secret_key: &str,
    timeout: std::time::Duration,
    org_slug: Option<&str>,
) -> Result<ZenClaims, AuthError> {
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| AuthError::BrowserFlowFailed(format!("failed to bind: {e}")))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or_else(|| AuthError::BrowserFlowFailed("no port".into()))?;

    // Generate a cryptographically random 16-byte hex state nonce for CSRF protection
    let mut nonce_bytes = [0u8; 16];
    getrandom::fill(&mut nonce_bytes)
        .map_err(|e| AuthError::BrowserFlowFailed(format!("failed to generate CSRF nonce: {e}")))?;
    let state: String = nonce_bytes.iter().map(|b| format!("{b:02x}")).collect();

    let redirect_url = format!("http://localhost:{port}/callback?state={state}");
    let mut sign_in_url = format!(
        "https://{clerk_frontend_api}/sign-in?redirect_url={redirect}",
        redirect = urlencoding::encode(&redirect_url)
    );
    if let Some(org) = org_slug {
        sign_in_url.push_str(&format!("&organization={}", urlencoding::encode(org)));
    }

    eprintln!("Opening browser to: {sign_in_url}");
    if let Err(error) = open::that(&sign_in_url) {
        eprintln!("Failed to open browser: {error}");
        eprintln!("Open the URL above manually, then return here.");
    }

    // Wait for callback — tiny_http::recv() blocks, so run in spawn_blocking
    let callback = tokio::task::spawn_blocking(move || wait_for_callback(server, timeout, state))
        .await
        .map_err(|e| AuthError::BrowserFlowFailed(format!("spawn_blocking join: {e}")))?
        ?;

    let jwt = match callback {
        CallbackResult::Jwt(jwt) => jwt,
        CallbackResult::SessionId(session_id) => {
            crate::api_key::mint_token_for_session(secret_key, &session_id).await?
        }
        CallbackResult::ClientToken(client_token) => {
            let session_id = crate::api_key::resolve_session_id_from_client_token(
                secret_key,
                &client_token,
            )
            .await?
            .ok_or_else(|| {
                AuthError::BrowserFlowFailed(
                    "could not resolve session id from Clerk callback token".into(),
                )
            })?;
            crate::api_key::mint_token_for_session(secret_key, &session_id).await?
        }
    };

    let claims = crate::jwks::validate(&jwt, secret_key).await?;
    crate::token_store::store(&jwt)?;
    Ok(claims)
}

enum CallbackResult {
    Jwt(String),
    SessionId(String),
    ClientToken(String),
}

/// Block until the callback server receives a request with a JWT.
///
/// Loops on `recv_timeout()`, ignoring requests that don't match `/callback?token=`.
/// This handles browser favicon requests, preflight requests, and user refreshes
/// that would otherwise cause a false failure.
fn wait_for_callback(
    server: tiny_http::Server,
    timeout: std::time::Duration,
    expected_state: String,
) -> Result<CallbackResult, AuthError> {
    let deadline = std::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(AuthError::BrowserFlowFailed(format!(
                "browser callback timed out after {}s",
                timeout.as_secs()
            )));
        }

        let request = match server.recv_timeout(remaining) {
            Ok(Some(req)) => req,
            Ok(None) => {
                return Err(AuthError::BrowserFlowFailed(format!(
                    "browser callback timed out after {}s",
                    timeout.as_secs()
                )));
            }
            Err(e) => {
                return Err(AuthError::BrowserFlowFailed(format!("recv error: {e}")));
            }
        };

        let url = request.url().to_string();

        // Ignore requests that aren't the callback (e.g., favicon, preflight)
        if !url.starts_with("/callback?") {
            let response = tiny_http::Response::from_string("").with_status_code(204);
            let _ = request.respond(response);
            continue;
        }

        // Extract token from query string BEFORE responding.
        // Clerk may redirect with different param names depending on configuration:
        //   - `token=<jwt>` (custom redirect page)
        //   - `__clerk_db_jwt=<jwt>` (Clerk hosted pages)
        //   - `session_token=<jwt>` (alternative Clerk configs)
        let Some(query) = url.split('?').nth(1) else {
            let err_response = tiny_http::Response::from_string(
                "<html><body><h1>Auth failed</h1><p>No token in callback. Check CLI output.</p></body></html>",
            )
            .with_header(tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap());
            let _ = request.respond(err_response);
            return Err(AuthError::BrowserFlowFailed("no query string in callback".into()));
        };

        let token_param_names = ["token", "__clerk_db_jwt", "session_token"];
        let session_id_param_names = [
            "session_id",
            "created_session_id",
            "__clerk_created_session",
            "sid",
        ];
        let mut token_candidates: Vec<(String, String)> = Vec::new();
        let mut session_id_candidates: Vec<(String, String)> = Vec::new();
        let mut all_params: Vec<(String, String)> = Vec::new();
        let mut found_state: Option<String> = None;
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let decoded = urlencoding::decode(value)
                    .map_err(|e| AuthError::BrowserFlowFailed(format!("URL decode: {e}")))?
                    .into_owned();

                all_params.push((key.to_string(), decoded.clone()));

                if token_param_names.contains(&key) {
                    token_candidates.push((key.to_string(), decoded.clone()));
                } else if session_id_param_names.contains(&key) {
                    session_id_candidates.push((key.to_string(), decoded.clone()));
                } else if key == "state" {
                    found_state = Some(decoded);
                }
            }
        }

        let found_jwt = token_candidates
            .iter()
            .map(|(_, token)| token)
            .find_map(|token| extract_jwt(token));

        if session_id_candidates.is_empty()
            && let Some((name, value)) = all_params
                .iter()
                .find(|(_, value)| value.starts_with("sess_"))
                .cloned()
        {
            session_id_candidates.push((name, value));
        }

        match found_jwt {
            Some(jwt) => {
                // Verify CSRF state nonce
                let state_ok = found_state
                    .as_deref()
                    .map_or(false, |s| s == expected_state);
                if !state_ok {
                    let response = tiny_http::Response::from_string(
                        "<html><body><h1>Auth failed</h1><p>State mismatch — possible CSRF attack. Check CLI output.</p></body></html>",
                    )
                    .with_header(tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap());
                    let _ = request.respond(response);
                    return Err(AuthError::BrowserFlowFailed(
                        "state mismatch — possible CSRF".into(),
                    ));
                }

                let response = tiny_http::Response::from_string(
                    "<html><body><h1>Authenticated!</h1><p>You can close this tab.</p></body></html>",
                )
                .with_header(tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap());
                let _ = request.respond(response);
                return Ok(CallbackResult::Jwt(jwt));
            }
            None => {
                if let Some((_, session_id)) = session_id_candidates
                    .iter()
                    .find(|(_, value)| value.starts_with("sess_"))
                    .cloned()
                {
                    let response = tiny_http::Response::from_string(
                        "<html><body><h1>Authenticated!</h1><p>Completing sign-in in CLI. You can close this tab.</p></body></html>",
                    )
                    .with_header(tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap());
                    let _ = request.respond(response);
                    return Ok(CallbackResult::SessionId(session_id));
                }

                if !token_candidates.is_empty() {
                    let (_, token) = token_candidates[0].clone();
                    let response = tiny_http::Response::from_string(
                        "<html><body><h1>Authenticated!</h1><p>Completing sign-in in CLI. You can close this tab.</p></body></html>",
                    )
                    .with_header(tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap());
                    let _ = request.respond(response);
                    return Ok(CallbackResult::ClientToken(token));
                }

                // No recognized token param — likely an intermediate Clerk redirect.
                // Respond with an informative page and keep waiting for the real callback.
                let response = tiny_http::Response::from_string(
                    "<html><body><h1>Waiting for authentication…</h1><p>Redirecting — please wait.</p></body></html>",
                )
                .with_header(tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap());
                let _ = request.respond(response);
                continue;
            }
        }
    }
}

fn looks_like_jwt(token: &str) -> bool {
    let mut parts = token.split('.');
    let (Some(a), Some(b), Some(c), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };

    !a.is_empty() && !b.is_empty() && !c.is_empty()
}

fn extract_jwt(token: &str) -> Option<String> {
    let trimmed = token.trim().trim_matches('"').trim_matches('\'');

    if looks_like_jwt(trimmed) {
        return Some(trimmed.to_string());
    }

    if let Some(rest) = trimmed.strip_prefix("Bearer ") {
        let rest = rest.trim();
        if looks_like_jwt(rest) {
            return Some(rest.to_string());
        }
    }

    None
}
