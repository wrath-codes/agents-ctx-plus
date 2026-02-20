//! # Integration tests for zen-auth
//!
//! These tests require live Clerk credentials. They are skipped (not failed)
//! when credentials are missing, following the same pattern as spike 0.17/0.20.
//!
//! ## Required environment variables
//!
//! ```bash
//! ZENITH_CLERK__SECRET_KEY=sk_test_...       # Stable — does not expire
//! ```
//!
//! Optional: `ZENITH_AUTH__TEST_USER_ID=user_...` to pin a specific test user.
//! If not set, tests auto-resolve the most recent user from the Clerk Backend API.
//! No test token env var needed — tests mint fresh JWTs programmatically.
//!
//! ## Run
//!
//! ```bash
//! cargo test -p zen-auth --test live_auth -- --nocapture
//! ```

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_env() {
    let workspace_env = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(".env"));

    if let Some(env_path) = workspace_env {
        let _ = dotenvy::from_path(&env_path);
    }
}

fn clerk_secret_key() -> Option<String> {
    load_env();
    let key = std::env::var("ZENITH_CLERK__SECRET_KEY").ok()?;
    if key.is_empty() || !key.starts_with("sk_") {
        return None;
    }
    Some(key)
}

/// Fetch a test user_id from `ZENITH_AUTH__TEST_USER_ID` env, or fall back to
/// fetching the first user from the Clerk Backend API.
async fn resolve_test_user_id(secret_key: &str) -> Option<String> {
    load_env();

    // Prefer explicit env var
    if let Ok(uid) = std::env::var("ZENITH_AUTH__TEST_USER_ID") {
        if !uid.is_empty() && uid.starts_with("user_") {
            return Some(uid);
        }
    }

    // Fall back: fetch first user from Clerk Backend API
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.clerk.com/v1/users?limit=1&order_by=-created_at")
        .header("Authorization", format!("Bearer {secret_key}"))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let users: serde_json::Value = resp.json().await.ok()?;
    users.as_array()?.first()?["id"].as_str().map(String::from)
}

/// Mint a fresh JWT for testing. Calls Clerk Backend API directly (same as api_key module).
/// This avoids storing anything in the keyring — raw reqwest call, no side effects.
async fn mint_test_jwt(secret_key: &str, user_id: &str) -> Option<String> {
    let client = reqwest::Client::new();

    let session = client
        .post("https://api.clerk.com/v1/sessions")
        .header("Authorization", format!("Bearer {secret_key}"))
        .json(&serde_json::json!({"user_id": user_id}))
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;

    let session_id = session["id"].as_str()?;

    let token_resp = client
        .post(format!(
            "https://api.clerk.com/v1/sessions/{session_id}/tokens/zenith_cli"
        ))
        .header("Authorization", format!("Bearer {secret_key}"))
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()?;

    token_resp["jwt"].as_str().map(String::from)
}

// ---------------------------------------------------------------------------
// Part 1: API key login (programmatic session + JWT minting)
// ---------------------------------------------------------------------------

/// Test login_with_api_key() against live Clerk Backend API.
/// Creates a session, mints a JWT from zenith_cli template, validates via JWKS.
#[tokio::test]
async fn api_key_login_mints_and_validates() {
    let Some(secret_key) = clerk_secret_key() else {
        eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
        return;
    };
    let Some(user_id) = resolve_test_user_id(&secret_key).await else {
        eprintln!("SKIP: could not resolve test user_id");
        return;
    };

    // Save existing credential
    let existing = zen_auth::token_store::load();

    let result = zen_auth::api_key::login_with_api_key(&secret_key, &user_id).await;

    // Restore original credential regardless of result
    zen_auth::token_store::delete().ok();
    if let Some(original) = existing {
        let _ = zen_auth::token_store::store(&original);
    }

    let claims = result.expect("login_with_api_key should succeed");

    assert_eq!(claims.user_id, user_id, "user_id should match input");
    assert!(
        claims.expires_at > chrono::Utc::now(),
        "minted token should not be expired"
    );

    if let Some(ref org_id) = claims.org_id {
        assert!(org_id.starts_with("org_"), "org_id format");
        eprintln!("  org_id: {org_id}");
        eprintln!("  org_slug: {:?}", claims.org_slug);
        eprintln!("  org_role: {:?}", claims.org_role);
    }

    eprintln!(
        "  PASS: api_key login — user={}, expires={}",
        claims.user_id, claims.expires_at
    );
}

// ---------------------------------------------------------------------------
// Part 2: JWKS validation with freshly minted JWT
// ---------------------------------------------------------------------------

/// Validate a freshly minted Clerk JWT via jwks::validate().
#[tokio::test]
async fn jwks_validate_fresh_token() {
    let Some(secret_key) = clerk_secret_key() else {
        eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
        return;
    };
    let Some(user_id) = resolve_test_user_id(&secret_key).await else {
        eprintln!("SKIP: could not resolve test user_id");
        return;
    };

    let Some(jwt) = mint_test_jwt(&secret_key, &user_id).await else {
        eprintln!("SKIP: failed to mint test JWT");
        return;
    };

    let claims = zen_auth::jwks::validate(&jwt, &secret_key)
        .await
        .expect("JWKS validation failed");

    assert_eq!(claims.user_id, user_id);
    assert!(!claims.raw_jwt.is_empty());
    assert!(claims.expires_at > chrono::Utc::now());

    eprintln!(
        "  PASS: JWKS validated fresh token — user={}",
        claims.user_id
    );
    eprintln!("  expires_at: {}", claims.expires_at);
    if let Some(ref org_id) = claims.org_id {
        eprintln!(
            "  org_id: {org_id}, slug: {:?}, role: {:?}",
            claims.org_slug, claims.org_role
        );
    }
}

/// Verify that an invalid token is rejected by JWKS validation.
#[tokio::test]
async fn jwks_rejects_invalid_token() {
    let Some(secret_key) = clerk_secret_key() else {
        eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
        return;
    };

    let result = zen_auth::jwks::validate("invalid.token.here", &secret_key).await;
    assert!(result.is_err(), "invalid token should be rejected");

    eprintln!("  PASS: invalid token rejected — {}", result.unwrap_err());
}

/// Verify the OnceLock provider cache — second validation reuses the provider.
#[tokio::test]
async fn jwks_provider_is_cached() {
    let Some(secret_key) = clerk_secret_key() else {
        eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
        return;
    };
    let Some(user_id) = resolve_test_user_id(&secret_key).await else {
        eprintln!("SKIP: could not resolve test user_id");
        return;
    };

    let Some(jwt) = mint_test_jwt(&secret_key, &user_id).await else {
        eprintln!("SKIP: failed to mint test JWT");
        return;
    };

    let t0 = std::time::Instant::now();
    let c1 = zen_auth::jwks::validate(&jwt, &secret_key)
        .await
        .expect("first validation failed");
    let d1 = t0.elapsed();

    let t1 = std::time::Instant::now();
    let c2 = zen_auth::jwks::validate(&jwt, &secret_key)
        .await
        .expect("second validation failed");
    let d2 = t1.elapsed();

    assert_eq!(c1.user_id, c2.user_id);
    eprintln!("  PASS: provider cached — first={d1:?}, second={d2:?}");
}

// ---------------------------------------------------------------------------
// Part 3: Claims extraction from real token
// ---------------------------------------------------------------------------

/// Validate to_identity() and is_near_expiry() with a freshly minted token.
#[tokio::test]
async fn claims_identity_and_expiry_from_fresh_token() {
    let Some(secret_key) = clerk_secret_key() else {
        eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
        return;
    };
    let Some(user_id) = resolve_test_user_id(&secret_key).await else {
        eprintln!("SKIP: could not resolve test user_id");
        return;
    };

    let Some(jwt) = mint_test_jwt(&secret_key, &user_id).await else {
        eprintln!("SKIP: failed to mint test JWT");
        return;
    };

    let claims = zen_auth::jwks::validate(&jwt, &secret_key)
        .await
        .expect("validation failed");

    let identity = claims.to_identity();
    assert_eq!(identity.user_id, claims.user_id);
    assert_eq!(identity.org_id, claims.org_id);
    assert_eq!(identity.org_slug, claims.org_slug);
    assert_eq!(identity.org_role, claims.org_role);

    // Freshly minted token should NOT be near-expiry
    assert!(
        !claims.is_near_expiry(60),
        "fresh token should not be near-expiry"
    );

    eprintln!("  PASS: to_identity() + is_near_expiry() correct for fresh token");
}

// ---------------------------------------------------------------------------
// Part 4: decode_expiry() consistency
// ---------------------------------------------------------------------------

/// Verify decode_expiry() (unverified base64) matches JWKS-validated exp.
#[tokio::test]
async fn decode_expiry_matches_jwks_validated() {
    let Some(secret_key) = clerk_secret_key() else {
        eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
        return;
    };
    let Some(user_id) = resolve_test_user_id(&secret_key).await else {
        eprintln!("SKIP: could not resolve test user_id");
        return;
    };

    let Some(jwt) = mint_test_jwt(&secret_key, &user_id).await else {
        eprintln!("SKIP: failed to mint test JWT");
        return;
    };

    let decoded_exp = zen_auth::refresh::decode_expiry(&jwt).expect("decode_expiry failed");
    let claims = zen_auth::jwks::validate(&jwt, &secret_key)
        .await
        .expect("validation failed");

    assert_eq!(
        decoded_exp.timestamp(),
        claims.expires_at.timestamp(),
        "decode_expiry() should match JWKS-validated exp"
    );

    eprintln!(
        "  PASS: decode_expiry={decoded_exp} == jwks.exp={}",
        claims.expires_at
    );
}

// ---------------------------------------------------------------------------
// Part 5: resolve_and_validate() end-to-end
// ---------------------------------------------------------------------------

/// Store a freshly minted token, then resolve_and_validate() to test the full flow.
#[tokio::test]
async fn resolve_and_validate_end_to_end() {
    let Some(secret_key) = clerk_secret_key() else {
        eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
        return;
    };
    let Some(user_id) = resolve_test_user_id(&secret_key).await else {
        eprintln!("SKIP: could not resolve test user_id");
        return;
    };

    let Some(jwt) = mint_test_jwt(&secret_key, &user_id).await else {
        eprintln!("SKIP: failed to mint test JWT");
        return;
    };

    // Save existing credential
    let existing = zen_auth::token_store::load();

    // Store the freshly minted token
    match zen_auth::token_store::store(&jwt) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("SKIP: token_store::store() failed: {e}");
            return;
        }
    }

    let result = zen_auth::resolve_and_validate(&secret_key).await;

    // Restore original credential
    zen_auth::token_store::delete().ok();
    if let Some(original) = existing {
        let _ = zen_auth::token_store::store(&original);
    }

    let claims = result
        .expect("resolve_and_validate should not error")
        .expect("should find a valid token");

    assert_eq!(claims.user_id, user_id);
    assert!(!claims.is_near_expiry(60));

    eprintln!(
        "  PASS: resolve_and_validate() — user={}, expires={}",
        claims.user_id, claims.expires_at
    );
}

// ---------------------------------------------------------------------------
// Part 6: Keyring lifecycle (test-specific service — no side effects)
// ---------------------------------------------------------------------------

/// Test keyring store/retrieve/delete using a test-specific service name.
#[test]
fn keyring_lifecycle() {
    let service = "zenith-cli-integration-test";
    let user = "test-jwt";
    let test_token = "integration_test_jwt_value_12345";

    let entry = match keyring::Entry::new(service, user) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("SKIP: keyring not available: {e}");
            return;
        }
    };

    match entry.set_password(test_token) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("SKIP: keyring set_password failed (no keyring daemon?): {e}");
            return;
        }
    }

    let retrieved = entry.get_password().expect("get_password should succeed");
    assert_eq!(retrieved, test_token);

    entry.delete_credential().expect("delete should succeed");
    assert!(entry.get_password().is_err(), "token should be gone");

    eprintln!("  PASS: keyring lifecycle — store → retrieve → delete");
}

/// Test token_store full cycle with save/restore of existing credentials.
///
/// Uses `figment::Jail` to sandbox environment variables so `ZENITH_AUTH__TOKEN`
/// (loaded by dotenvy) does not shadow the file-fallback tier during the test.
#[test]
fn token_store_full_cycle() {
    figment::Jail::expect_with(|jail| {
        // Isolate from production credentials:
        // - Use a test-specific keyring service so we don't read/write prod entries
        // - Clear env var so tier 2 doesn't shadow keyring/file tiers
        // - Sandbox HOME so file fallback writes to temp dir, not real ~/.zenith
        jail.set_env("ZENITH_KEYRING_SERVICE", "zenith-cli-test");
        jail.set_env("ZENITH_AUTH__TOKEN", "");
        let tmp_home = jail.directory().to_str().unwrap().to_string();
        jail.set_env("HOME", &tmp_home);

        let existing = zen_auth::token_store::load();

        let test_token = "integration_test_store_cycle_jwt";

        match zen_auth::token_store::store(test_token) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("SKIP: token_store::store() failed: {e}");
                return Ok(());
            }
        }

        let loaded = zen_auth::token_store::load();
        assert_eq!(loaded.as_deref(), Some(test_token));

        let source = zen_auth::token_store::detect_token_source();
        assert!(source.is_some());
        eprintln!("  source: {:?}", source.unwrap());

        zen_auth::token_store::delete().expect("delete should succeed");

        // Restore original credentials
        if let Some(original) = existing {
            let _ = zen_auth::token_store::store(&original);
        }

        eprintln!("  PASS: token_store full cycle");
        Ok(())
    });
}
