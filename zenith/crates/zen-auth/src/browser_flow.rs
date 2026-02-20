use crate::claims::ZenClaims;
use crate::error::AuthError;

/// Execute the browser-based Clerk login flow.
///
/// 1. Start `tiny_http` on `127.0.0.1:0` (random port)
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

    let redirect_url = format!("http://127.0.0.1:{port}/callback?state={state}");
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
    let jwt = tokio::task::spawn_blocking(move || wait_for_callback(server, timeout, state))
        .await
        .map_err(|e| AuthError::BrowserFlowFailed(format!("spawn_blocking join: {e}")))?
        ?;

    let claims = crate::jwks::validate(&jwt, secret_key).await?;
    crate::token_store::store(&jwt)?;
    Ok(claims)
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
) -> Result<String, AuthError> {
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
        let mut found_jwt: Option<String> = None;
        let mut found_state: Option<String> = None;
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if token_param_names.contains(&key) {
                    let jwt = urlencoding::decode(value)
                        .map_err(|e| AuthError::BrowserFlowFailed(format!("URL decode: {e}")))?;
                    found_jwt = Some(jwt.into_owned());
                } else if key == "state" {
                    let st = urlencoding::decode(value)
                        .map_err(|e| AuthError::BrowserFlowFailed(format!("URL decode: {e}")))?;
                    found_state = Some(st.into_owned());
                }
            }
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
                return Ok(jwt);
            }
            None => {
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
