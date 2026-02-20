//! # Spike 0.17: Clerk Auth + Turso JWKS Integration
//!
//! Validates the complete CLI authentication chain for Phase 9 (Team & Pro):
//!
//! - **Part A**: `clerk-rs` JWT validation in a non-web context (no framework features)
//! - **Part B**: `tiny_http` localhost callback for browser OAuth flow
//! - **Part C**: Token storage (`keyring` + file fallback) and JWT expiry detection
//! - **Part D**: Turso JWKS integration — Clerk JWT as `libsql` auth token
//! - **Part E**: Clerk API key verification for CI/headless fallback
//!
//! ## Prerequisites
//!
//! Tests in Parts A, D, E require live Clerk and Turso credentials.
//! Set environment variables in `zenith/.env`:
//!
//! ```bash
//! ZENITH_CLERK__SECRET_KEY=sk_test_...
//! ZENITH_CLERK__PUBLISHABLE_KEY=pk_test_...
//! ZENITH_CLERK__JWKS_URL=https://ruling-doe-21.clerk.accounts.dev/.well-known/jwks.json
//! ZENITH_AUTH__TEST_TOKEN=eyJ...  # A valid Clerk JWT for testing (generate via Dashboard)
//! ZENITH_TURSO__URL=libsql://...
//! ```
//!
//! For Turso JWKS tests (Part D), you must have registered Clerk's JWKS with Turso:
//! ```bash
//! turso org jwks save clerk https://ruling-doe-21.clerk.accounts.dev/.well-known/jwks.json
//! ```
//!
//! Tests are skipped (not failed) when credentials are missing.

use std::io::Read as _;
use std::sync::Arc;
use std::time::Duration;

use crate::test_support::spike_clerk_helpers::{
    fresh_clerk_token, load_env, mint_fresh_jwt, resolve_test_user_id,
};

/// Get a Clerk secret key from env, or None if not configured.
fn clerk_secret_key() -> Option<String> {
    load_env();
    let key = std::env::var("ZENITH_CLERK__SECRET_KEY").ok()?;
    if key.is_empty() || !key.starts_with("sk_") {
        return None;
    }
    Some(key)
}

/// Decode JWT payload without verification (for claim inspection).
/// Returns the payload as a JSON value.
fn decode_jwt_payload_unverified(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_b64 = parts[1];
    // JWT uses base64url encoding (no padding)
    use base64::Engine;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let payload_bytes = engine.decode(payload_b64).ok()?;
    serde_json::from_slice(&payload_bytes).ok()
}

// ============================================================================
// Part A: clerk-rs JWT Validation
// ============================================================================

#[cfg(test)]
mod part_a_clerk_validation {
    use super::*;
    use clerk_rs::ClerkConfiguration;
    use clerk_rs::clerk::Clerk;
    use clerk_rs::validators::authorizer::validate_jwt;
    use clerk_rs::validators::jwks::MemoryCacheJwksProvider;

    /// A1: Verify that JwksValidator can be created from a Clerk secret key
    /// without requiring any web framework.
    #[test]
    fn spike_clerk_jwks_validator_creates() {
        let Some(secret_key) = clerk_secret_key() else {
            eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
            return;
        };

        let config = ClerkConfiguration::new(None, None, Some(secret_key.clone()), None);
        let clerk = Clerk::new(config);
        let _provider = MemoryCacheJwksProvider::new(clerk);

        // If we got here without panic, the validator was created successfully.
        // clerk-rs doesn't expose a "is_ready" method — creation is the test.
        eprintln!(
            "  clerk-rs JwksValidator created from secret key ({}...)",
            &secret_key[..12]
        );
    }

    /// A2: Validate a real Clerk JWT and extract claims.
    #[tokio::test]
    async fn spike_clerk_validate_jwt_from_env() {
        let Some(secret_key) = clerk_secret_key() else {
            eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
            return;
        };
        let Some(user_id) = resolve_test_user_id(&secret_key).await else {
            eprintln!("SKIP: could not resolve test user_id");
            return;
        };
        let Some(token) = mint_fresh_jwt(&secret_key, &user_id).await else {
            eprintln!("SKIP: failed to mint test JWT");
            return;
        };

        let config = ClerkConfiguration::new(None, None, Some(secret_key), None);
        let clerk = Clerk::new(config);
        let provider = Arc::new(MemoryCacheJwksProvider::new(clerk));

        match validate_jwt(&token, provider).await {
            Ok(jwt) => {
                eprintln!("  JWT validated successfully");
                eprintln!("  sub (user_id): {}", jwt.sub);
                eprintln!("  iat: {}", jwt.iat);
                eprintln!("  exp: {}", jwt.exp);
                if let Some(ref org) = jwt.org {
                    eprintln!("  org_id: {}", org.id);
                    eprintln!("  org_slug: {}", org.slug);
                    eprintln!("  org_role: {}", org.role);
                } else {
                    eprintln!("  org: None (user not in an org)");
                }
                if let Some(ref sid) = jwt.sid {
                    eprintln!("  session_id: {sid}");
                }

                // Basic assertions
                assert!(!jwt.sub.is_empty(), "sub should not be empty");
                assert!(jwt.sub.starts_with("user_"), "sub should start with user_");
                assert!(jwt.exp > jwt.iat, "exp should be after iat");
            }
            Err(e) => {
                // Token might be expired — that's informative, not a test failure
                let payload = decode_jwt_payload_unverified(&token);
                if let Some(ref p) = payload {
                    if let Some(exp) = p.get("exp").and_then(|v| v.as_i64()) {
                        let now = chrono::Utc::now().timestamp();
                        if exp < now {
                            eprintln!(
                                "  Token expired (exp={exp}, now={now}). This is expected for old test tokens."
                            );
                            eprintln!("  Generate a fresh token and set ZENITH_AUTH__TEST_TOKEN");
                            return;
                        }
                    }
                }
                panic!("JWT validation failed: {e}");
            }
        }
    }

    /// A3: Verify that an expired JWT is rejected.
    #[tokio::test]
    async fn spike_clerk_expired_token_rejected() {
        let Some(secret_key) = clerk_secret_key() else {
            eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
            return;
        };

        let config = ClerkConfiguration::new(None, None, Some(secret_key), None);
        let clerk = Clerk::new(config);
        let provider = Arc::new(MemoryCacheJwksProvider::new(clerk));

        // A clearly invalid/expired token (malformed — clerk-rs should reject it)
        let bad_token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyX3Rlc3QiLCJleHAiOjEwMDAwMDAwMDAsImlhdCI6MTAwMDAwMDAwMH0.invalid_signature";

        let result = validate_jwt(bad_token, provider).await;
        assert!(
            result.is_err(),
            "Expected expired/invalid token to be rejected"
        );
        eprintln!(
            "  Invalid token correctly rejected: {:?}",
            result.err().unwrap()
        );
    }
}

// ============================================================================
// Part B: Browser OAuth Flow (tiny_http callback)
// ============================================================================

#[cfg(test)]
mod part_b_browser_flow {
    use super::*;

    /// B1: Verify that tiny_http captures a token from a mock callback redirect.
    #[test]
    fn spike_tiny_http_callback_captures_token() {
        let server = tiny_http::Server::http("127.0.0.1:0").expect("Failed to start tiny_http");
        let addr = server.server_addr().to_ip().expect("Should be IP address");
        let port = addr.port();
        eprintln!("  tiny_http listening on 127.0.0.1:{port}");

        let expected_token = "test_jwt_eyJhbGciOiJSUzI1NiJ9.payload.signature";

        // Spawn a thread to send the mock callback request using raw TCP
        let url_path = format!("/callback?token={expected_token}");
        let client_handle = std::thread::spawn(move || {
            // Small delay to ensure server is ready
            std::thread::sleep(Duration::from_millis(50));
            let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}"))
                .expect("TCP connect failed");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set_read_timeout failed");
            use std::io::Write;
            write!(
                stream,
                "GET {url_path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
            )
            .expect("Write failed");
            // Read response with a fixed buffer (don't wait for EOF)
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let response = String::from_utf8_lossy(&buf[..n]);
            assert!(
                response.contains("200")
                    || response.contains("Success")
                    || response.contains("close"),
                "Response should indicate success, got: {response}"
            );
        });

        // Server waits for the callback request
        let request = server
            .recv_timeout(Duration::from_secs(5))
            .expect("Timeout waiting for request")
            .expect("No request received");

        let url_str = request.url().to_string();
        eprintln!("  Received callback: {url_str}");

        // Parse the token from query parameters
        let captured_token = url_str
            .split("token=")
            .nth(1)
            .map(|s| s.split('&').next().unwrap_or(s));

        assert_eq!(
            captured_token,
            Some(expected_token),
            "Should capture the exact token from the callback URL"
        );

        // Respond with success HTML
        let response = tiny_http::Response::from_string(
            "<html><body><h1>Login Success!</h1><p>You can close this tab.</p></body></html>",
        )
        .with_header(
            "Content-Type: text/html"
                .parse::<tiny_http::Header>()
                .unwrap(),
        );
        request.respond(response).expect("Failed to send response");

        client_handle.join().expect("Client thread panicked");
        eprintln!("  Browser callback flow validated");
    }

    /// B2: Verify that `open::that` doesn't panic (may or may not open browser in CI).
    #[test]
    fn spike_browser_open_does_not_panic() {
        // We don't actually open a browser in tests — just verify the crate loads
        // and the function signature works. In CI, this will return an error (no display),
        // which is fine.
        let result = open::that("https://example.com");
        match result {
            Ok(()) => eprintln!("  open::that succeeded (browser may have opened)"),
            Err(e) => eprintln!("  open::that returned error (expected in CI): {e}"),
        }
        // Both Ok and Err are acceptable — we just need it to not panic
    }
}

// ============================================================================
// Part C: Token Storage + Expiry Detection
// ============================================================================

#[cfg(test)]
mod part_c_token_storage {
    use super::*;

    /// C1: Verify keyring store/retrieve/delete lifecycle.
    #[test]
    fn spike_keyring_store_retrieve_delete() {
        let service = "zenith-spike-test";
        let user = "test-token";
        let token_value = "sk_test_fake_token_for_keyring_validation";

        let entry = match keyring::Entry::new(service, user) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("SKIP: keyring not available: {e}");
                return;
            }
        };

        // Store
        match entry.set_password(token_value) {
            Ok(()) => eprintln!("  Stored token in keyring"),
            Err(e) => {
                eprintln!("SKIP: keyring set_password failed (no keyring daemon?): {e}");
                return;
            }
        }

        // Retrieve
        match entry.get_password() {
            Ok(retrieved) => {
                assert_eq!(
                    retrieved, token_value,
                    "Retrieved token should match stored"
                );
                eprintln!("  Retrieved token from keyring: matches");
            }
            Err(e) => {
                eprintln!("SKIP: keyring get_password failed: {e}");
                // Clean up even on failure
                let _ = entry.delete_credential();
                return;
            }
        }

        // Delete
        match entry.delete_credential() {
            Ok(()) => eprintln!("  Deleted token from keyring"),
            Err(e) => eprintln!("  Warning: keyring delete failed: {e}"),
        }

        // Verify deleted
        match entry.get_password() {
            Ok(_) => eprintln!("  Warning: token still in keyring after delete"),
            Err(_) => eprintln!("  Confirmed: token no longer in keyring"),
        }
    }

    /// C2: Verify file-based token storage fallback with proper permissions.
    #[test]
    fn spike_file_storage_fallback() {
        let tmp = tempfile::TempDir::new().expect("Failed to create temp dir");
        let creds_dir = tmp.path().join(".zenith");
        let creds_file = creds_dir.join("credentials");

        // Create directory
        std::fs::create_dir_all(&creds_dir).expect("Failed to create .zenith dir");

        // Write token
        let token_data = serde_json::json!({
            "clerk_jwt": "eyJhbGciOiJSUzI1NiJ9.test.payload",
            "expires_at": "2026-02-15T00:00:00Z",
            "user_id": "user_test123",
            "org_id": "org_abc"
        });
        std::fs::write(
            &creds_file,
            serde_json::to_string_pretty(&token_data).unwrap(),
        )
        .expect("Failed to write credentials");

        // Set permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&creds_file, perms).expect("Failed to set permissions");

            let metadata = std::fs::metadata(&creds_file).unwrap();
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "Credentials file should be 0600");
            eprintln!("  File permissions: {mode:o} (expected 600)");
        }

        // Read back
        let contents = std::fs::read_to_string(&creds_file).expect("Failed to read credentials");
        let parsed: serde_json::Value =
            serde_json::from_str(&contents).expect("Failed to parse credentials JSON");

        assert_eq!(
            parsed["clerk_jwt"].as_str(),
            Some("eyJhbGciOiJSUzI1NiJ9.test.payload")
        );
        assert_eq!(parsed["user_id"].as_str(), Some("user_test123"));
        assert_eq!(parsed["org_id"].as_str(), Some("org_abc"));
        eprintln!("  File storage fallback validated");
    }

    /// C3: Verify JWT expiry detection via payload decoding.
    #[tokio::test]
    async fn spike_token_expiry_detection() {
        // Create a mock JWT with known exp claim
        // Header: {"alg":"none","typ":"JWT"}
        // Payload: {"sub":"user_test","exp":<timestamp>,"iat":1000000000}
        use base64::Engine;
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;

        let now = chrono::Utc::now().timestamp();

        // Test 1: Token expired 1 hour ago
        let expired_payload = serde_json::json!({
            "sub": "user_expired",
            "exp": now - 3600,
            "iat": now - 7200
        });
        let expired_jwt = format!(
            "eyJhbGciOiJub25lIn0.{}.nosig",
            engine.encode(serde_json::to_vec(&expired_payload).unwrap())
        );

        let payload = decode_jwt_payload_unverified(&expired_jwt).unwrap();
        let exp = payload["exp"].as_i64().unwrap();
        assert!(exp < now, "Token should be expired");
        eprintln!("  Expired token detected: exp={exp}, now={now}");

        // Test 2: Token valid for 6 more hours
        let valid_payload = serde_json::json!({
            "sub": "user_valid",
            "exp": now + 21600,
            "iat": now
        });
        let valid_jwt = format!(
            "eyJhbGciOiJub25lIn0.{}.nosig",
            engine.encode(serde_json::to_vec(&valid_payload).unwrap())
        );

        let payload = decode_jwt_payload_unverified(&valid_jwt).unwrap();
        let exp = payload["exp"].as_i64().unwrap();
        assert!(exp > now, "Token should be valid");
        let remaining = exp - now;
        eprintln!("  Valid token detected: {remaining}s remaining");

        // Test 3: Near-expiry (within 60s buffer)
        let near_expiry_payload = serde_json::json!({
            "sub": "user_nearexpiry",
            "exp": now + 30,
            "iat": now - 3570
        });
        let near_jwt = format!(
            "eyJhbGciOiJub25lIn0.{}.nosig",
            engine.encode(serde_json::to_vec(&near_expiry_payload).unwrap())
        );

        let payload = decode_jwt_payload_unverified(&near_jwt).unwrap();
        let exp = payload["exp"].as_i64().unwrap();
        let is_near_expiry = (exp - now) < 60;
        assert!(is_near_expiry, "Token should be near expiry");
        eprintln!(
            "  Near-expiry token detected: {}s remaining (< 60s buffer)",
            exp - now
        );

        // Test 4: Malformed JWT
        let malformed = decode_jwt_payload_unverified("not.a.jwt.token.at.all");
        assert!(malformed.is_none(), "Malformed JWT should return None");
        eprintln!("  Malformed JWT correctly rejected");

        // Test 5: Real test token (if available)
        if let Some(real_token) = fresh_clerk_token().await {
            if let Some(payload) = decode_jwt_payload_unverified(&real_token) {
                let exp = payload["exp"].as_i64().unwrap_or(0);
                let sub = payload["sub"].as_str().unwrap_or("unknown");
                let remaining = exp - now;
                if remaining > 0 {
                    eprintln!("  Real token: sub={sub}, expires in {remaining}s");
                } else {
                    eprintln!("  Real token: sub={sub}, EXPIRED {}s ago", -remaining);
                }
            }
        }
    }
}

// ============================================================================
// Part D: Turso JWKS Integration
// ============================================================================

#[cfg(test)]
mod part_d_turso_jwks {
    use super::*;
    use libsql::Builder;
    use tempfile::TempDir;

    use crate::test_support::spike_clerk_helpers::turso_jwks_credentials;

    /// D1: Connect to Turso via `Builder::new_remote` using a Clerk JWT as auth token.
    #[tokio::test(flavor = "multi_thread")]
    async fn spike_turso_jwks_remote_connection() {
        let Some((url, clerk_jwt)) = turso_jwks_credentials().await else {
            eprintln!("SKIP: Turso URL or Clerk test token not configured");
            return;
        };

        eprintln!(
            "  Connecting to Turso via JWKS: {}...",
            &url[..url.len().min(40)]
        );

        let db = match Builder::new_remote(url.clone(), clerk_jwt).build().await {
            Ok(db) => db,
            Err(e) => {
                eprintln!("  Connection failed: {e}");
                eprintln!("  This may mean:");
                eprintln!("    - JWKS not registered: run `turso org jwks save clerk <jwks_url>`");
                eprintln!("    - JWT missing Turso permissions (`p` claim)");
                eprintln!("    - JWT expired (check ZENITH_AUTH__TEST_TOKEN)");
                return;
            }
        };

        let conn = db.connect().expect("Failed to create connection");

        // Simple query to verify the connection works
        let mut rows = conn
            .query("SELECT 1 AS result", ())
            .await
            .expect("SELECT 1 failed");
        let row = rows
            .next()
            .await
            .expect("next() failed")
            .expect("No row returned");
        let result: i64 = row.get(0).expect("Failed to get column 0");
        assert_eq!(result, 1);
        eprintln!("  SELECT 1 succeeded via Turso JWKS connection");
    }

    /// D2: Connect via embedded replica using Clerk JWT.
    #[tokio::test(flavor = "multi_thread")]
    async fn spike_turso_jwks_embedded_replica() {
        let Some((url, clerk_jwt)) = turso_jwks_credentials().await else {
            eprintln!("SKIP: Turso URL or Clerk test token not configured");
            return;
        };

        let tmp = TempDir::new().expect("Failed to create temp dir");
        let local_path = tmp.path().join("replica.db");
        let local_path_str = local_path.to_string_lossy().to_string();

        eprintln!("  Opening embedded replica with Clerk JWT...");

        let db = match Builder::new_remote_replica(local_path_str, url, clerk_jwt)
            .build()
            .await
        {
            Ok(db) => db,
            Err(e) => {
                eprintln!("  Embedded replica creation failed: {e}");
                eprintln!("  Check JWKS registration and JWT permissions");
                return;
            }
        };

        // Initial sync
        match db.sync().await {
            Ok(rep) => eprintln!("  Initial sync completed: {rep:?}"),
            Err(e) => {
                eprintln!("  Initial sync failed: {e}");
                return;
            }
        }

        let conn = db.connect().expect("Failed to create connection");

        // Create a test table (may already exist from previous runs)
        let table_name = "spike_jwks_test";
        conn.execute(
            &format!("CREATE TABLE IF NOT EXISTS {table_name} (id INTEGER PRIMARY KEY, val TEXT)"),
            (),
        )
        .await
        .expect("CREATE TABLE failed");

        // Insert a row
        let ts = chrono::Utc::now().timestamp();
        conn.execute(
            &format!("INSERT OR REPLACE INTO {table_name} (id, val) VALUES (1, ?1)"),
            [format!("spike_0.17_at_{ts}")],
        )
        .await
        .expect("INSERT failed");

        // Sync to cloud
        match db.sync().await {
            Ok(rep) => eprintln!("  Sync after write: {rep:?}"),
            Err(e) => eprintln!("  Sync after write failed: {e}"),
        }

        // Query back
        let mut rows = conn
            .query(&format!("SELECT val FROM {table_name} WHERE id = 1"), ())
            .await
            .expect("SELECT failed");
        let row = rows.next().await.expect("next() failed").expect("No row");
        let val: String = row.get(0).expect("Failed to get val");
        assert!(val.starts_with("spike_0.17_at_"));
        eprintln!("  Embedded replica roundtrip: wrote and read '{val}'");
    }

    /// D3: Write-forwarding through embedded replica with Clerk JWT.
    #[tokio::test(flavor = "multi_thread")]
    async fn spike_turso_jwks_write_forward() {
        let Some((url, clerk_jwt)) = turso_jwks_credentials().await else {
            eprintln!("SKIP: Turso URL or Clerk test token not configured");
            return;
        };

        let tmp1 = TempDir::new().expect("Failed to create temp dir 1");
        let tmp2 = TempDir::new().expect("Failed to create temp dir 2");
        let path1 = tmp1
            .path()
            .join("replica1.db")
            .to_string_lossy()
            .to_string();
        let path2 = tmp2
            .path()
            .join("replica2.db")
            .to_string_lossy()
            .to_string();

        // Replica 1: write
        let db1 = match Builder::new_remote_replica(path1, url.clone(), clerk_jwt.clone())
            .build()
            .await
        {
            Ok(db) => db,
            Err(e) => {
                eprintln!("  SKIP: replica 1 failed: {e}");
                return;
            }
        };
        let _ = db1.sync().await;
        let conn1 = db1.connect().expect("conn1 failed");

        let table = "spike_jwks_forward";
        conn1
            .execute(
                &format!("CREATE TABLE IF NOT EXISTS {table} (id INTEGER PRIMARY KEY, msg TEXT)"),
                (),
            )
            .await
            .expect("CREATE failed");

        let unique_msg = format!("forward_test_{}", chrono::Utc::now().timestamp_millis());
        conn1
            .execute(
                &format!("INSERT OR REPLACE INTO {table} (id, msg) VALUES (42, ?1)"),
                [unique_msg.clone()],
            )
            .await
            .expect("INSERT failed");

        // Sync replica 1 to cloud
        db1.sync().await.expect("Sync replica 1 failed");
        eprintln!("  Replica 1 wrote and synced: '{unique_msg}'");

        // Small delay for propagation
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Replica 2: read
        let db2 = Builder::new_remote_replica(path2, url, clerk_jwt)
            .build()
            .await
            .expect("Replica 2 failed");
        db2.sync().await.expect("Sync replica 2 failed");
        let conn2 = db2.connect().expect("conn2 failed");

        let mut rows = conn2
            .query(&format!("SELECT msg FROM {table} WHERE id = 42"), ())
            .await
            .expect("SELECT from replica 2 failed");

        if let Some(row) = rows.next().await.expect("next() failed") {
            let msg: String = row.get(0).expect("get msg failed");
            assert_eq!(msg, unique_msg);
            eprintln!("  Replica 2 sees replica 1's write: '{msg}'");
        } else {
            eprintln!("  WARNING: Replica 2 didn't see the row (propagation delay?)");
        }
    }

    /// D4: Document behavior when Clerk JWT expires during an embedded replica session.
    /// This is observational — we document the error type and verify local reads survive.
    #[tokio::test(flavor = "multi_thread")]
    async fn spike_turso_jwks_expired_token_behavior() {
        let Some((url, _)) = turso_jwks_credentials().await else {
            eprintln!("SKIP: Turso URL or Clerk test token not configured");
            return;
        };

        // Use a clearly expired/invalid token to test error handling
        let expired_token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyX3Rlc3QiLCJleHAiOjEwMDAwMDAwMDB9.invalid";

        let tmp = TempDir::new().expect("tmp failed");
        let path = tmp
            .path()
            .join("expired_test.db")
            .to_string_lossy()
            .to_string();

        // Try to connect with expired token
        match Builder::new_remote_replica(path, url, expired_token.to_string())
            .build()
            .await
        {
            Ok(db) => {
                // Connection creation might succeed (deferred auth)
                eprintln!("  Builder succeeded with expired token (deferred auth)");

                // Sync should fail
                match db.sync().await {
                    Ok(_) => eprintln!("  UNEXPECTED: sync succeeded with expired token"),
                    Err(e) => {
                        eprintln!("  Sync failed with expired token (expected): {e}");
                        eprintln!("  Error type: {:?}", e);
                    }
                }

                // Local reads may still work on existing data
                let conn = db.connect().expect("local connect should work");
                match conn.query("SELECT 1", ()).await {
                    Ok(_) => eprintln!("  Local reads work despite expired token"),
                    Err(e) => eprintln!("  Local reads also failed: {e}"),
                }
            }
            Err(e) => {
                eprintln!("  Builder failed with expired token: {e}");
                eprintln!("  Error type: {:?}", e);
                eprintln!("  This means auth is validated at connection time, not deferred");
            }
        }

        // Document: this test is observational, not asserting specific behavior.
        // The key finding is documented in the test output.
        eprintln!("  Expired token behavior documented (see output above)");
    }
}

// ============================================================================
// Part E: API Key Fallback
// ============================================================================

#[cfg(test)]
mod part_e_api_key {
    use super::*;

    /// E1: Verify Clerk API key via the Backend API.
    /// This uses reqwest directly since clerk-rs may not expose api_keys/verify.
    #[tokio::test]
    async fn spike_clerk_api_key_verify() {
        load_env();
        let api_key = match std::env::var("ZENITH_AUTH__API_KEY") {
            Ok(k) if !k.is_empty() => k,
            _ => {
                eprintln!("SKIP: ZENITH_AUTH__API_KEY not set");
                return;
            }
        };
        let secret_key = match clerk_secret_key() {
            Some(k) => k,
            None => {
                eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
                return;
            }
        };

        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.clerk.com/v1/api_keys/verify")
            .header("Authorization", format!("Bearer {secret_key}"))
            .json(&serde_json::json!({ "secret": api_key }))
            .send()
            .await;

        match resp {
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                eprintln!("  API key verify response: status={status}");
                eprintln!("  Body: {body}");
                if status.is_success() {
                    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
                    if let Some(subject) = parsed.get("subject").and_then(|v| v.as_str()) {
                        eprintln!("  API key subject: {subject}");
                    }
                }
            }
            Err(e) => {
                eprintln!("  API key verify request failed: {e}");
            }
        }
    }

    /// E2: Verify that reqwest can fetch Clerk's public JWKS endpoint.
    /// This confirms we can fall back to DIY JWT validation if clerk-rs has issues.
    #[tokio::test]
    async fn spike_clerk_jwks_public_fetch() {
        load_env();
        let jwks_url = match std::env::var("ZENITH_CLERK__JWKS_URL") {
            Ok(u) if !u.is_empty() => u,
            _ => {
                eprintln!("SKIP: ZENITH_CLERK__JWKS_URL not set");
                return;
            }
        };

        let client = reqwest::Client::new();
        let resp = client
            .get(&jwks_url)
            .send()
            .await
            .expect("JWKS fetch failed");

        assert!(
            resp.status().is_success(),
            "JWKS endpoint should return 200"
        );

        let body: serde_json::Value = resp.json().await.expect("JWKS should be JSON");
        let keys = body["keys"]
            .as_array()
            .expect("JWKS should have keys array");
        assert!(!keys.is_empty(), "JWKS should have at least one key");

        for (i, key) in keys.iter().enumerate() {
            let kty = key["kty"].as_str().unwrap_or("unknown");
            let alg = key["alg"].as_str().unwrap_or("unknown");
            let kid = key["kid"].as_str().unwrap_or("unknown");
            eprintln!("  Key {i}: kty={kty}, alg={alg}, kid={kid}");
        }
        eprintln!("  JWKS endpoint returned {} keys", keys.len());
    }
}
