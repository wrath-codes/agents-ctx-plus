//! Shared test utilities for zen-db integration tests.

#[cfg(test)]
pub(crate) mod helpers {
    use zen_core::identity::AuthIdentity;

    use crate::ZenDb;
    use crate::service::ZenService;
    use crate::trail::writer::TrailWriter;

    /// Create an in-memory ZenService with trail disabled (for pure DB tests).
    pub async fn test_service() -> ZenService {
        let db = ZenDb::open_local(":memory:").await.unwrap();
        ZenService::from_db(db, TrailWriter::disabled(), None)
    }

    /// Create an in-memory ZenService with trail enabled writing to a temp dir.
    pub async fn test_service_with_trail(trail_dir: std::path::PathBuf) -> ZenService {
        let db = ZenDb::open_local(":memory:").await.unwrap();
        let trail = TrailWriter::new(trail_dir).unwrap();
        ZenService::from_db(db, trail, None)
    }

    /// Create an in-memory ZenService with a specific identity (for visibility tests).
    pub async fn test_service_with_identity(identity: AuthIdentity) -> ZenService {
        let db = ZenDb::open_local(":memory:").await.unwrap();
        ZenService::from_db(db, TrailWriter::disabled(), Some(identity))
    }

    /// Start a session and return its ID (convenience for tests that need a session).
    pub async fn start_test_session(svc: &ZenService) -> String {
        let (session, _) = svc.start_session().await.unwrap();
        session.id
    }
}

#[cfg(test)]
pub(crate) mod spike_clerk_helpers {
    /// Load .env from the workspace root (zenith/.env).
    pub fn load_env() {
        let workspace_env = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join(".env"));
        if let Some(env_path) = workspace_env {
            let _ = dotenvy::from_path(&env_path);
        }
    }

    /// Get Turso URL from env.
    pub fn turso_url() -> Option<String> {
        load_env();
        let url = std::env::var("ZENITH_TURSO__URL").ok()?;
        if url.is_empty() {
            return None;
        }
        Some(url)
    }

    /// Resolve a test user_id from env or by fetching the first user from Clerk.
    pub async fn resolve_test_user_id(secret_key: &str) -> Option<String> {
        load_env();
        if let Ok(uid) = std::env::var("ZENITH_AUTH__TEST_USER_ID") {
            if !uid.is_empty() && uid.starts_with("user_") {
                return Some(uid);
            }
        }

        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.clerk.com/v1/users?limit=1&order_by=-created_at")
            .header("Authorization", format!("Bearer {secret_key}"))
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            eprintln!("  resolve_test_user_id: Clerk API returned {}", resp.status());
            return None;
        }
        let users: serde_json::Value = resp.json().await.ok()?;
        users.as_array()?.first()?["id"].as_str().map(String::from)
    }

    /// Mint a fresh Clerk JWT for testing via Backend API.
    /// Creates a session, generates a JWT from the zenith_cli template,
    /// then revokes the session to avoid zombie session accumulation.
    pub async fn mint_fresh_jwt(secret_key: &str, user_id: &str) -> Option<String> {
        let client = reqwest::Client::new();

        let resp = client
            .post("https://api.clerk.com/v1/sessions")
            .header("Authorization", format!("Bearer {secret_key}"))
            .json(&serde_json::json!({"user_id": user_id}))
            .send()
            .await;
        let session = match resp {
            Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.ok()?,
            Ok(r) => {
                eprintln!("  mint_fresh_jwt: session creation failed: {}", r.status());
                return None;
            }
            Err(e) => {
                eprintln!("  mint_fresh_jwt: session request failed: {e}");
                return None;
            }
        };

        let session_id = session["id"].as_str()?;

        let token_resp = client
            .post(format!(
                "https://api.clerk.com/v1/sessions/{session_id}/tokens/zenith_cli"
            ))
            .header("Authorization", format!("Bearer {secret_key}"))
            .send()
            .await;
        let jwt = match token_resp {
            Ok(r) if r.status().is_success() => {
                let body = r.json::<serde_json::Value>().await.ok()?;
                body["jwt"].as_str().map(String::from)
            }
            Ok(r) => {
                eprintln!("  mint_fresh_jwt: token generation failed: {}", r.status());
                None
            }
            Err(e) => {
                eprintln!("  mint_fresh_jwt: token request failed: {e}");
                None
            }
        };

        // Revoke the session to avoid accumulating zombie sessions.
        // The JWT is self-contained and remains valid independent of session state.
        let _ = client
            .post(format!(
                "https://api.clerk.com/v1/sessions/{session_id}/revoke"
            ))
            .header("Authorization", format!("Bearer {secret_key}"))
            .send()
            .await;

        jwt
    }

    /// Get a fresh Clerk JWT: mint via Backend API if secret key is available,
    /// fall back to static `ZENITH_AUTH__TEST_TOKEN` env var.
    pub async fn fresh_clerk_token() -> Option<String> {
        load_env();
        let secret_key = std::env::var("ZENITH_CLERK__SECRET_KEY").ok()?;
        if secret_key.is_empty() || !secret_key.starts_with("sk_") {
            // No secret key — fall back to static token
            let token = std::env::var("ZENITH_AUTH__TEST_TOKEN").ok()?;
            return if token.is_empty() { None } else { Some(token) };
        }

        let user_id = resolve_test_user_id(&secret_key).await?;
        let jwt = mint_fresh_jwt(&secret_key, &user_id).await;
        if jwt.is_some() {
            return jwt;
        }

        // Minting failed — fall back to static token
        eprintln!("  fresh_clerk_token: minting failed, falling back to ZENITH_AUTH__TEST_TOKEN");
        let token = std::env::var("ZENITH_AUTH__TEST_TOKEN").ok()?;
        if token.is_empty() { None } else { Some(token) }
    }

    /// Turso credentials using a freshly minted Clerk JWT (JWKS path).
    /// Mints a new JWT via Backend API so tokens are never stale.
    pub async fn turso_jwks_credentials() -> Option<(String, String)> {
        let url = turso_url()?;
        let token = fresh_clerk_token().await?;
        Some((url, token))
    }

}
