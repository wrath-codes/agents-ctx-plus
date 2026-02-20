//! # Spike 0.20: Turso Catalog + Clerk Visibility Scoping
//!
//! Validates Turso as the global `indexed_packages` catalog with Clerk JWT-driven
//! visibility scoping (public/team/private). Tests embedded replicas for local
//! catalog access, three-tier search federation, and operational concerns.
//!
//! ## Production Architecture
//!
//! ```text
//! Turso Cloud: zenith_global
//!   └── indexed_packages (catalog with visibility scoping)
//!       Embedded replica on every authenticated user's machine
//!
//! R2: Lance datasets (search data)
//!   └── Paths discovered via Turso catalog
//!
//! Search flow:
//!   1. Turso replica → SELECT r2_lance_path WHERE visibility scoped
//!   2. DuckDB lance extension → lance_vector_search(path, ...)
//! ```
//!
//! ## Prerequisites
//!
//! Requires live Turso + Clerk credentials in `zenith/.env`:
//!
//! ```bash
//! ZENITH_TURSO__URL=libsql://...
//! ZENITH_TURSO__PLATFORM_API_KEY=...
//! ZENITH_TURSO__ORG_SLUG=...
//! ZENITH_AUTH__TEST_TOKEN=eyJ...   # Valid Clerk JWT
//! ```
//!
//! Tests are skipped when credentials are missing.

use libsql::Builder;
use tempfile::TempDir;

use crate::retry::{self, RetryConfig};
use crate::test_support::spike_clerk_helpers::{load_env, turso_jwks_credentials};

fn is_spike_transient_turso_error(e: &libsql::Error) -> bool {
    if retry::is_transient_turso_error(e) {
        return true;
    }

    let msg = e.to_string().to_ascii_lowercase();
    msg.contains("file is not a database") || msg.contains("database table is locked")
}

// ============================================================================
// Helpers (spike 0.20-specific)
// ============================================================================

/// Execute a Turso operation using the production retry module.
/// Skips the test if all retries are exhausted on transient infra errors.
/// Non-transient errors panic immediately.
macro_rules! turso_op {
    ($expr:expr) => {{
        let __cfg = RetryConfig::default();
        let mut __delay = __cfg.base_delay;
        let mut __result = $expr;
        for __attempt in 1..=__cfg.max_attempts {
            match __result {
                Ok(ref _v) => break,
                Err(ref e) if is_spike_transient_turso_error(e) && __attempt < __cfg.max_attempts => {
                    eprintln!(
                        "  Turso transient (attempt {__attempt}/{}), retrying in {__delay:?}: {e}",
                        __cfg.max_attempts
                    );
                    tokio::time::sleep(__delay).await;
                    __delay = std::cmp::min(__delay * 2, __cfg.max_delay);
                    __result = $expr;
                }
                Err(ref e) if is_spike_transient_turso_error(e) => {
                    eprintln!("SKIP: Turso transient infra error after {} attempts: {e}", __cfg.max_attempts);
                    return;
                }
                Err(_) => break,
            }
        }
        __result.unwrap()
    }};
}

/// Validate Clerk JWT and extract typed claims (aether pattern).
/// Uses `clerk-rs` JWKS validation — not raw JWT decoding.
async fn validate_clerk_token(token: &str) -> Option<ClerkClaims> {
    load_env();
    let secret_key = std::env::var("ZENITH_CLERK__SECRET_KEY").ok()?;
    if secret_key.is_empty() || !secret_key.starts_with("sk_") {
        return None;
    }

    let config = clerk_rs::ClerkConfiguration::new(None, None, Some(secret_key), None);
    let clerk = clerk_rs::clerk::Clerk::new(config);
    let jwks_provider = std::sync::Arc::new(
        clerk_rs::validators::jwks::MemoryCacheJwksProvider::new(clerk),
    );

    let jwt = clerk_rs::validators::authorizer::validate_jwt(token, jwks_provider)
        .await
        .ok()?;

    Some(ClerkClaims {
        sub: jwt.sub,
        org_id: jwt.org.as_ref().map(|o| o.id.clone()),
        org_role: jwt.org.as_ref().map(|o| o.role.clone()),
    })
}

/// Minimal claims struct — mirrors aether's Claims but only the fields we need.
#[derive(Debug)]
struct ClerkClaims {
    sub: String,
    org_id: Option<String>,
    org_role: Option<String>,
}

/// Generate a fresh Turso Platform API auth token for a specific DB.
async fn turso_platform_token(db_name: &str) -> Option<String> {
    load_env();
    let api_key = std::env::var("ZENITH_TURSO__PLATFORM_API_KEY").ok()?;
    let org = std::env::var("ZENITH_TURSO__ORG_SLUG").ok()?;
    if api_key.is_empty() || org.is_empty() {
        return None;
    }

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.turso.tech/v1/organizations/{org}/databases/{db_name}/auth/tokens?expiration=1h&authorization=full-access"
    );
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    body["jwt"].as_str().map(|s| s.to_string())
}

/// Create a temporary Turso DB via Platform API. Returns (url, db_name).
async fn create_temp_turso_db(name_prefix: &str) -> Option<(String, String)> {
    load_env();
    let api_key = std::env::var("ZENITH_TURSO__PLATFORM_API_KEY").ok()?;
    let org = std::env::var("ZENITH_TURSO__ORG_SLUG").ok()?;
    if api_key.is_empty() || org.is_empty() {
        return None;
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let db_name = format!("{name_prefix}-{ts}");

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!(
            "https://api.turso.tech/v1/organizations/{org}/databases"
        ))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "name": db_name,
            "group": "default"
        }))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        eprintln!("  Failed to create temp DB: {status} {body}");
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;
    let hostname = body["database"]["Hostname"]
        .as_str()
        .or_else(|| body["database"]["hostname"].as_str())?;
    let url = format!("libsql://{hostname}");

    eprintln!("  Created temp DB: {db_name} at {url}");
    Some((url, db_name))
}

/// Delete a Turso DB via Platform API.
async fn delete_turso_db(db_name: &str) {
    load_env();
    let api_key = std::env::var("ZENITH_TURSO__PLATFORM_API_KEY").unwrap_or_default();
    let org = std::env::var("ZENITH_TURSO__ORG_SLUG").unwrap_or_default();
    if api_key.is_empty() || org.is_empty() {
        return;
    }

    let client = reqwest::Client::new();
    let _ = client
        .delete(&format!(
            "https://api.turso.tech/v1/organizations/{org}/databases/{db_name}"
        ))
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await;
    eprintln!("  Deleted temp DB: {db_name}");
}

/// Unique table name to avoid collisions between test runs.
fn unique_table(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{prefix}_{ts}")
}

/// The indexed_packages CREATE TABLE SQL.
fn indexed_packages_ddl(table: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {table} (
            ecosystem TEXT NOT NULL,
            package TEXT NOT NULL,
            version TEXT NOT NULL,
            visibility TEXT NOT NULL DEFAULT 'public',
            owner_id TEXT,
            team_id TEXT,
            r2_lance_path TEXT,
            repo_url TEXT,
            description TEXT,
            symbol_count INTEGER DEFAULT 0,
            chunk_count INTEGER DEFAULT 0,
            indexed_by TEXT NOT NULL,
            indexed_at TEXT NOT NULL,
            schema_version INTEGER DEFAULT 1,
            PRIMARY KEY (ecosystem, package, version)
        )"
    )
}

// ============================================================================
// Part J: Turso Catalog + Visibility
// ============================================================================

/// J0: Programmatic org-scoped JWT generation + clerk-rs validation.
///
/// Validates the full flow that `znt auth` will need:
/// 1. Create a Clerk session via Backend API (reqwest)
/// 2. Generate JWT from zenith_cli template with org context
/// 3. Validate JWT with clerk-rs (JWKS)
/// 4. Extract org claims (org_id, org_slug, org_role)
///
/// This proves we can generate org-scoped JWTs without a browser flow,
/// which is required for CI/headless and for the server component that
/// will mint R2 temp credentials.
#[tokio::test(flavor = "multi_thread")]
async fn spike_programmatic_org_jwt() {
    load_env();
    let secret_key = match std::env::var("ZENITH_CLERK__SECRET_KEY") {
        Ok(k) if k.starts_with("sk_") => k,
        _ => {
            eprintln!("SKIP: ZENITH_CLERK__SECRET_KEY not set");
            return;
        }
    };

    let client = reqwest::Client::new();

    // 1. Create a session for the test user
    let resp = client
        .post("https://api.clerk.com/v1/sessions")
        .header("Authorization", format!("Bearer {secret_key}"))
        .json(&serde_json::json!({
            "user_id": "user_39PB2iMuMcpYGrHobrukpqZ8UjE"
        }))
        .send()
        .await
        .unwrap();

    if !resp.status().is_success() {
        eprintln!("SKIP: Failed to create session: {}", resp.status());
        return;
    }

    let session: serde_json::Value = resp.json().await.unwrap();
    let session_id = session["id"].as_str().expect("session id");
    let org_id_from_session = session["last_active_organization_id"].as_str();
    eprintln!("  Session: {session_id}");
    eprintln!("  Active org: {org_id_from_session:?}");

    // 2. Generate JWT from zenith_cli template
    let resp = client
        .post(&format!(
            "https://api.clerk.com/v1/sessions/{session_id}/tokens/zenith_cli"
        ))
        .header("Authorization", format!("Bearer {secret_key}"))
        .send()
        .await
        .unwrap();

    if !resp.status().is_success() {
        eprintln!("SKIP: Failed to get JWT: {}", resp.status());
        return;
    }

    let token_resp: serde_json::Value = resp.json().await.unwrap();
    let jwt = token_resp["jwt"].as_str().expect("jwt");
    eprintln!("  JWT length: {}", jwt.len());

    // 3. Validate with clerk-rs (JWKS)
    let config = clerk_rs::ClerkConfiguration::new(None, None, Some(secret_key.clone()), None);
    let clerk = clerk_rs::clerk::Clerk::new(config);
    let jwks_provider = std::sync::Arc::new(
        clerk_rs::validators::jwks::MemoryCacheJwksProvider::new(clerk),
    );

    let clerk_jwt = clerk_rs::validators::authorizer::validate_jwt(jwt, jwks_provider)
        .await
        .expect("JWT validation failed");

    eprintln!("  sub: {}", clerk_jwt.sub);

    // 4. Extract org claims — clerk-rs uses #[serde(flatten)] so org claims
    // may be in the `org` field OR in `other` depending on how the JWT was structured.
    // Custom JWT templates put org_id/org_slug/org_role as top-level claims,
    // which clerk-rs's flatten should pick up into ActiveOrganization.
    eprintln!("  org field: {:?}", clerk_jwt.org);
    eprintln!(
        "  other keys: {:?}",
        clerk_jwt.other.keys().collect::<Vec<_>>()
    );

    // Try org field first, fall back to other map
    let (org_id, org_slug, org_role) = if let Some(ref org) = clerk_jwt.org {
        (org.id.clone(), org.slug.clone(), org.role.clone())
    } else {
        // Custom templates may put these as top-level claims in `other`
        let oid = clerk_jwt
            .other
            .get("org_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let oslug = clerk_jwt
            .other
            .get("org_slug")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let orole = clerk_jwt
            .other
            .get("org_role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        (oid, oslug, orole)
    };

    eprintln!("  org_id: {org_id}");
    eprintln!("  org_slug: {org_slug}");
    eprintln!("  org_role: {org_role}");

    assert!(!org_id.is_empty(), "org_id should not be empty");
    assert!(org_id.starts_with("org_"), "org_id should start with org_");
    assert!(!org_slug.is_empty(), "org_slug should not be empty");
    assert_eq!(org_role, "org:admin", "Test user should be org admin");

    // Verify Turso permissions are also present
    let turso_p = clerk_jwt
        .other
        .get("p")
        .expect("JWT should have Turso 'p' claim");
    assert!(turso_p.get("rw").is_some(), "Should have rw permissions");

    eprintln!(
        "  PASS: programmatic org-scoped JWT — session → template → clerk-rs validates → org claims extracted"
    );
}

/// J1: Create indexed_packages in Turso with visibility columns. INSERT + SELECT.
#[tokio::test(flavor = "multi_thread")]
async fn spike_turso_indexed_packages_schema() {
    let Some((url, token)) = turso_jwks_credentials().await else {
        eprintln!("SKIP: Turso/Clerk credentials not configured");
        return;
    };

    let db = match Builder::new_remote(url.clone(), token).build().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Turso connection failed: {e}");
            return;
        }
    };
    let conn = db.connect().unwrap();

    let table = unique_table("idx_pkg");
    eprintln!("  Creating table: {table}");

    // Create schema
    conn.execute(&indexed_packages_ddl(&table), ())
        .await
        .unwrap();

    // Insert public, team, and private rows
    conn.execute(
        &format!(
            "INSERT INTO {table} (ecosystem, package, version, visibility, owner_id, team_id,
             r2_lance_path, symbol_count, indexed_by, indexed_at) VALUES
            ('rust', 'tokio', '1.49.0', 'public', NULL, NULL,
             's3://zenith/lance/rust/tokio/1.49.0', 1234, 'user_aaa', '2026-02-08T00:00:00Z'),
            ('rust', 'internal-sdk', '2.0.0', 'team', NULL, 'org_acme',
             's3://zenith/lance/rust/internal-sdk/2.0.0', 500, 'user_bbb', '2026-02-08T00:00:00Z'),
            ('rust', 'my-app', '0.1.0', 'private', 'user_aaa', NULL,
             's3://zenith/lance/rust/my-app/0.1.0', 200, 'user_aaa', '2026-02-08T00:00:00Z')"
        ),
        (),
    )
    .await
    .unwrap();

    // Query: public only (anonymous user)
    let mut rows = conn
        .query(
            &format!("SELECT package FROM {table} WHERE visibility = 'public' ORDER BY package"),
            (),
        )
        .await
        .unwrap();
    let mut public_pkgs = vec![];
    while let Some(row) = rows.next().await.unwrap() {
        public_pkgs.push(row.get::<String>(0).unwrap());
    }
    assert_eq!(public_pkgs, vec!["tokio"]);
    eprintln!("  Public only: {public_pkgs:?}");

    // Query: team member of org_acme (sees public + team)
    let mut rows = conn
        .query(
            &format!(
                "SELECT package FROM {table} WHERE
             visibility = 'public'
             OR (visibility = 'team' AND team_id = 'org_acme')
             ORDER BY package"
            ),
            (),
        )
        .await
        .unwrap();
    let mut team_pkgs = vec![];
    while let Some(row) = rows.next().await.unwrap() {
        team_pkgs.push(row.get::<String>(0).unwrap());
    }
    assert_eq!(team_pkgs, vec!["internal-sdk", "tokio"]);
    eprintln!("  Team (org_acme): {team_pkgs:?}");

    // Query: user_aaa (sees public + own private)
    let mut rows = conn
        .query(
            &format!(
                "SELECT package FROM {table} WHERE
             visibility = 'public'
             OR (visibility = 'private' AND owner_id = 'user_aaa')
             ORDER BY package"
            ),
            (),
        )
        .await
        .unwrap();
    let mut owner_pkgs = vec![];
    while let Some(row) = rows.next().await.unwrap() {
        owner_pkgs.push(row.get::<String>(0).unwrap());
    }
    assert_eq!(owner_pkgs, vec!["my-app", "tokio"]);
    eprintln!("  Owner (user_aaa): {owner_pkgs:?}");

    // Cleanup
    conn.execute(&format!("DROP TABLE {table}"), ())
        .await
        .unwrap();
    eprintln!("  PASS: indexed_packages schema + visibility scoping works in Turso");
}

/// J2: Embedded replica syncs the catalog correctly.
#[tokio::test(flavor = "multi_thread")]
async fn spike_turso_catalog_embedded_replica() {
    let Some((url, token)) = turso_jwks_credentials().await else {
        eprintln!("SKIP: Turso/Clerk credentials not configured");
        return;
    };

    let table = unique_table("idx_repl");

    // Write via remote connection
    let db = match Builder::new_remote(url.clone(), token.clone())
        .build()
        .await
    {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Turso connection failed: {e}");
            return;
        }
    };
    let conn = db.connect().unwrap();
    turso_op!(conn.execute(&indexed_packages_ddl(&table), ()).await);
    turso_op!(conn.execute(
        &format!(
            "INSERT INTO {table} (ecosystem, package, version, visibility,
             r2_lance_path, symbol_count, indexed_by, indexed_at) VALUES
            ('rust', 'serde', '1.0.0', 'public',
             's3://zenith/lance/rust/serde/1.0.0', 800, 'user_test', '2026-02-08T00:00:00Z')"
        ),
        (),
    )
    .await);
    drop(conn);
    drop(db);
    eprintln!("  Wrote catalog row via remote connection");

    // Read via embedded replica
    let tmp = TempDir::new().unwrap();
    let local_path = tmp
        .path()
        .join("replica_j2.db")
        .to_string_lossy()
        .to_string();

    let replica_db = match Builder::new_remote_replica(local_path, url.clone(), token)
        .read_your_writes(true)
        .build()
        .await
    {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Embedded replica creation failed: {e}");
            return;
        }
    };
    turso_op!(replica_db.sync().await);

    let conn = replica_db.connect().unwrap();
    let (pkg, count) = {
        let mut rows = turso_op!(conn
            .query(
                &format!("SELECT package, symbol_count FROM {table} WHERE package = 'serde'"),
                (),
            )
            .await);
        let row = turso_op!(rows.next().await).expect("Should find serde row");
        let pkg: String = row.get(0).unwrap();
        let count: i64 = row.get(1).unwrap();
        (pkg, count)
    };
    assert_eq!(pkg, "serde");
    assert_eq!(count, 800);

    // Cleanup
    turso_op!(conn.execute(&format!("DROP TABLE {table}"), ()).await);
    turso_op!(replica_db.sync().await);
    eprintln!("  PASS: embedded replica syncs catalog — found serde with {count} symbols");
}

/// J3: Clerk JWT claims drive visibility-scoped queries.
///
/// ## Claim Patting (Production Requirement)
///
/// The current test JWT only has `sub` (user_id) and `p` (Turso permissions).
/// It does NOT have `org_id`/`org_slug`/`org_role` because it was generated
/// without an organization context.
///
/// For production, the `zenith_cli` JWT template in Clerk must be configured
/// to include org claims when the user has an active organization. The
/// `znt auth login` flow needs to request an org-scoped session token.
///
/// This test validates:
/// - `sub` claim drives private visibility (works with current token)
/// - Team visibility SQL logic works (tested with hardcoded org_id)
/// - The pattern: extract claims → build WHERE clause → execute
///
/// What remains for Phase 9:
/// - Clerk org creation for testing
/// - JWT template with org claims (`org_id`, `org_slug`, `org_role`)
/// - `znt auth login` requesting org-scoped sessions
#[tokio::test(flavor = "multi_thread")]
async fn spike_clerk_jwt_visibility_scoping() {
    let Some((url, token)) = turso_jwks_credentials().await else {
        eprintln!("SKIP: Turso/Clerk credentials not configured");
        return;
    };

    // Validate JWT via Clerk JWKS and extract typed claims (aether pattern)
    let claims = match validate_clerk_token(&token).await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: Clerk JWT validation failed");
            return;
        }
    };
    let user_id = &claims.sub;
    let org_id = claims.org_id.as_deref().unwrap_or("none");
    let org_role = claims.org_role.as_deref().unwrap_or("none");

    eprintln!("  JWT sub={user_id}, org_id={org_id}, org_role={org_role}");

    if org_id == "none" {
        eprintln!("  WARNING: JWT has no org_id — team visibility tests will use hardcoded values");
        eprintln!(
            "  To fix: update zenith_cli JWT template with org claims and regenerate test token"
        );
    }

    let db = match Builder::new_remote(url.clone(), token).build().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Turso connection failed: {e}");
            return;
        }
    };
    let conn = db.connect().unwrap();

    let table = unique_table("idx_vis");
    conn.execute(&indexed_packages_ddl(&table), ())
        .await
        .unwrap();

    // Insert: public + team (owned by user's org) + private (owned by user) + private (other)
    conn.execute(
        &format!(
            "INSERT INTO {table} (ecosystem, package, version, visibility, owner_id, team_id,
             r2_lance_path, indexed_by, indexed_at) VALUES
            ('rust', 'tokio', '1.49.0', 'public', NULL, NULL,
             's3://zenith/lance/rust/tokio/1.49.0', '{user_id}', '2026-02-08T00:00:00Z'),
            ('rust', 'team-sdk', '1.0.0', 'team', NULL, '{org_id}',
             's3://zenith/lance/rust/team-sdk/1.0.0', '{user_id}', '2026-02-08T00:00:00Z'),
            ('rust', 'my-code', '0.1.0', 'private', '{user_id}', NULL,
             's3://zenith/lance/rust/my-code/0.1.0', '{user_id}', '2026-02-08T00:00:00Z'),
            ('rust', 'other-code', '0.1.0', 'private', 'user_someone_else', NULL,
             's3://zenith/lance/rust/other-code/0.1.0', 'user_someone_else', '2026-02-08T00:00:00Z'),
            ('rust', 'other-team', '1.0.0', 'team', NULL, 'org_other',
             's3://zenith/lance/rust/other-team/1.0.0', 'user_other', '2026-02-08T00:00:00Z')"
        ),
        (),
    ).await.unwrap();

    // Full visibility query using ALL claims from JWT (sub + org_id)
    // This is the exact query `znt search` will run in production
    let mut rows = conn
        .query(
            &format!(
                "SELECT package FROM {table} WHERE
             visibility = 'public'
             OR (visibility = 'team' AND team_id = ?1)
             OR (visibility = 'private' AND owner_id = ?2)
             ORDER BY package"
            ),
            libsql::params![org_id.to_string(), user_id.to_string()],
        )
        .await
        .unwrap();

    let mut visible = vec![];
    while let Some(row) = rows.next().await.unwrap() {
        visible.push(row.get::<String>(0).unwrap());
    }

    assert!(
        visible.contains(&"tokio".to_string()),
        "Should see public package"
    );
    assert!(
        visible.contains(&"my-code".to_string()),
        "Should see own private package"
    );
    assert!(
        !visible.contains(&"other-code".to_string()),
        "Should NOT see other's private package"
    );
    assert!(
        !visible.contains(&"other-team".to_string()),
        "Should NOT see other team's package"
    );

    // Team visibility only works if JWT has org_id
    if org_id != "none" {
        assert!(
            visible.contains(&"team-sdk".to_string()),
            "Should see own team's package"
        );
        eprintln!("  Team visibility verified with real org_id={org_id}");
    } else {
        eprintln!("  Team visibility skipped (no org_id in JWT)");
    }

    eprintln!("  Visible to {user_id}: {visible:?}");

    // Cleanup
    conn.execute(&format!("DROP TABLE {table}"), ())
        .await
        .unwrap();
    eprintln!("  PASS: Clerk JWT sub claim drives visibility — no custom RBAC needed");
}

/// J4: Full E2E: Turso catalog → Lance path → DuckDB search.
#[tokio::test(flavor = "multi_thread")]
async fn spike_catalog_to_lance_search_e2e() {
    let Some((url, token)) = turso_jwks_credentials().await else {
        eprintln!("SKIP: Turso/Clerk credentials not configured");
        return;
    };

    use arrow_array::types::Float32Type;
    use arrow_array::{FixedSizeListArray, StringArray};
    use arrow_array::{RecordBatch as RecordBatch57, RecordBatchIterator};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;

    // 1. Write a small Lance dataset locally
    let tmp = TempDir::new().unwrap();
    let lance_dir = tmp.path().join("lance_e2e");
    let lance_uri = lance_dir.to_str().unwrap();

    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("doc_comment", DataType::Utf8, true),
        Field::new(
            "embedding",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 384),
            false,
        ),
    ]));

    fn synth_emb(seed: u32) -> Vec<f32> {
        (0..384)
            .map(|i| ((seed as f32) / 100.0 + (i as f32) / 384.0).sin())
            .collect()
    }

    let names = StringArray::from(vec!["spawn", "sleep", "block_on"]);
    let docs = StringArray::from(vec![
        Some("Spawns a new async task"),
        Some("Sleeps for the given duration"),
        Some("Blocks the current thread on a future"),
    ]);
    let embeddings: Vec<Option<Vec<Option<f32>>>> = (0..3)
        .map(|i| Some(synth_emb(i).into_iter().map(Some).collect()))
        .collect();
    let emb_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(embeddings, 384);

    let batch = RecordBatch57::try_new(
        schema.clone(),
        vec![Arc::new(names), Arc::new(docs), Arc::new(emb_array)],
    )
    .unwrap();

    let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
    let lance_db = lancedb::connect(lance_uri).execute().await.unwrap();
    lance_db
        .create_table("symbols", Box::new(batches))
        .execute()
        .await
        .unwrap();

    let dataset_path = lance_dir
        .join("symbols.lance")
        .to_string_lossy()
        .to_string();
    eprintln!("  Wrote Lance dataset: {dataset_path}");

    // 2. Insert catalog row into Turso
    let turso_db = match Builder::new_remote(url.clone(), token).build().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Turso connection failed: {e}");
            return;
        }
    };
    let conn = turso_db.connect().unwrap();

    let table = unique_table("idx_e2e");
    turso_op!(conn.execute(&indexed_packages_ddl(&table), ()).await);
    turso_op!(conn.execute(
        &format!(
            "INSERT INTO {table} (ecosystem, package, version, visibility,
             r2_lance_path, symbol_count, indexed_by, indexed_at) VALUES
            ('rust', 'tokio', '1.49.0', 'public', ?1, 3, 'test', '2026-02-08T00:00:00Z')"
        ),
        [dataset_path.as_str()],
    )
    .await);

    // 3. Query Turso to get lance path
    let mut rows = turso_op!(conn
        .query(
            &format!("SELECT r2_lance_path FROM {table} WHERE package = 'tokio'"),
            (),
        )
        .await);
    let row = turso_op!(rows.next().await).expect("Should find tokio");
    let lance_path: String = row.get(0).unwrap();

    eprintln!("  Turso catalog → lance path: {lance_path}");

    // 4. DuckDB: search the Lance dataset
    let duckdb_conn = duckdb::Connection::open_in_memory().unwrap();
    duckdb_conn
        .execute_batch("INSTALL lance FROM community; LOAD lance;")
        .unwrap();

    let query_emb = synth_emb(0); // should match "spawn"
    let query_sql: String = format!(
        "[{}]",
        query_emb
            .iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut stmt = duckdb_conn
        .prepare(&format!(
            "SELECT name, _distance
         FROM lance_vector_search('{lance_path}', 'embedding', {query_sql}::FLOAT[384], k=3)
         ORDER BY _distance ASC"
        ))
        .unwrap();

    let results: Vec<(String, f64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].0, "spawn", "Nearest should be 'spawn'");
    assert!(results[0].1 < 0.01);

    eprintln!(
        "  DuckDB search: {} → distance={:.6}",
        results[0].0, results[0].1
    );

    // Cleanup
    turso_op!(conn.execute(&format!("DROP TABLE {table}"), ()).await);
    eprintln!("  PASS: Turso catalog → Lance path → DuckDB search — full E2E works");
}

// ============================================================================
// Part K: Three-Tier Search
// ============================================================================

/// K1: Three-tier search returns correctly scoped results.
#[tokio::test(flavor = "multi_thread")]
async fn spike_three_tier_search() {
    let Some((url, token)) = turso_jwks_credentials().await else {
        eprintln!("SKIP: Turso/Clerk credentials not configured");
        return;
    };

    use arrow_array::types::Float32Type;
    use arrow_array::{FixedSizeListArray, StringArray};
    use arrow_array::{RecordBatch as RecordBatch57, RecordBatchIterator};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;

    fn synth_emb(seed: u32) -> Vec<f32> {
        (0..384)
            .map(|i| ((seed as f32) / 100.0 + (i as f32) / 384.0).sin())
            .collect()
    }

    let tmp = TempDir::new().unwrap();

    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 384),
            false,
        ),
    ]));

    // Write 3 Lance datasets: public, team, private
    let mut lance_paths = vec![];
    for (label, names, seed_base) in [
        ("public", vec!["pub_func_a", "pub_func_b"], 0u32),
        ("team", vec!["team_func_a", "team_func_b"], 100),
        ("private", vec!["priv_func_a", "priv_func_b"], 200),
    ] {
        let dir = tmp.path().join(format!("lance_{label}"));
        let uri = dir.to_str().unwrap();

        let name_arr = StringArray::from(names.clone());
        let embs: Vec<Option<Vec<Option<f32>>>> = (0..names.len() as u32)
            .map(|i| Some(synth_emb(seed_base + i).into_iter().map(Some).collect()))
            .collect();
        let emb_arr = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(embs, 384);
        let batch =
            RecordBatch57::try_new(schema.clone(), vec![Arc::new(name_arr), Arc::new(emb_arr)])
                .unwrap();

        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());
        let db = lancedb::connect(uri).execute().await.unwrap();
        db.create_table("symbols", Box::new(batches))
            .execute()
            .await
            .unwrap();
        lance_paths.push(dir.join("symbols.lance").to_string_lossy().to_string());
    }

    eprintln!("  Wrote 3 Lance datasets (public, team, private)");

    // Insert catalog rows
    let turso_db = match Builder::new_remote(url.clone(), token).build().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Turso connection failed: {e}");
            return;
        }
    };
    let conn = turso_db.connect().unwrap();

    let table = unique_table("idx_3tier");
    turso_op!(conn.execute(&indexed_packages_ddl(&table), ()).await);
    turso_op!(conn.execute(
        &format!(
            "INSERT INTO {table} (ecosystem, package, version, visibility, team_id, owner_id,
             r2_lance_path, indexed_by, indexed_at) VALUES
            ('rust', 'pub-lib', '1.0.0', 'public', NULL, NULL, ?1, 'test', '2026-02-08T00:00:00Z'),
            ('rust', 'team-lib', '1.0.0', 'team', 'org_acme', NULL, ?2, 'test', '2026-02-08T00:00:00Z'),
            ('rust', 'priv-lib', '1.0.0', 'private', NULL, 'user_owner', ?3, 'test', '2026-02-08T00:00:00Z')"
        ),
        libsql::params![lance_paths[0].as_str(), lance_paths[1].as_str(), lance_paths[2].as_str()],
    ).await);

    // Query as team member of org_acme → should see public + team (NOT private)
    let mut rows = turso_op!(conn
        .query(
            &format!(
                "SELECT package, r2_lance_path FROM {table} WHERE
             visibility = 'public'
             OR (visibility = 'team' AND team_id = 'org_acme')
             ORDER BY package"
            ),
            (),
        )
        .await);

    let mut team_paths = vec![];
    while let Some(row) = turso_op!(rows.next().await) {
        let pkg: String = row.get(0).unwrap();
        let path: String = row.get(1).unwrap();
        eprintln!("  Team member sees: {pkg}");
        team_paths.push((pkg, path));
    }

    assert_eq!(team_paths.len(), 2, "Team member should see 2 packages");
    assert!(team_paths.iter().any(|(p, _)| p == "pub-lib"));
    assert!(team_paths.iter().any(|(p, _)| p == "team-lib"));

    // Search across visible Lance datasets
    let duckdb_conn = duckdb::Connection::open_in_memory().unwrap();
    duckdb_conn
        .execute_batch("INSTALL lance FROM community; LOAD lance;")
        .unwrap();

    let query_emb = synth_emb(0); // matches pub_func_a
    let query_sql: String = format!(
        "[{}]",
        query_emb
            .iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut all_results: Vec<(String, f64)> = vec![];
    for (pkg, path) in &team_paths {
        let mut stmt = duckdb_conn
            .prepare(&format!(
                "SELECT name, _distance
             FROM lance_vector_search('{path}', 'embedding', {query_sql}::FLOAT[384], k=2)
             ORDER BY _distance ASC"
            ))
            .unwrap();
        let results: Vec<(String, f64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for r in results {
            eprintln!("    [{pkg}] {} → dist={:.6}", r.0, r.1);
            all_results.push(r);
        }
    }

    // Sort merged results by distance
    all_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    assert!(!all_results.is_empty());
    assert_eq!(
        all_results[0].0, "pub_func_a",
        "Best match should be pub_func_a"
    );

    // Cleanup
    turso_op!(conn.execute(&format!("DROP TABLE {table}"), ()).await);
    eprintln!("  PASS: three-tier search — team member sees public + team, results merged");
}

/// K2: Private code indexing — only owner can discover.
#[tokio::test(flavor = "multi_thread")]
async fn spike_private_code_indexing() {
    let Some((url, token)) = turso_jwks_credentials().await else {
        eprintln!("SKIP: Turso/Clerk credentials not configured");
        return;
    };

    let claims = match validate_clerk_token(&token).await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: Clerk JWT validation failed");
            return;
        }
    };
    let user_id = &claims.sub;

    let db = match Builder::new_remote(url.clone(), token).build().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Turso connection failed: {e}");
            return;
        }
    };
    let conn = db.connect().unwrap();

    let table = unique_table("idx_priv");
    turso_op!(conn.execute(&indexed_packages_ddl(&table), ()).await);

    // Insert private package owned by the JWT user
    turso_op!(conn.execute(
        &format!(
            "INSERT INTO {table} (ecosystem, package, version, visibility, owner_id,
             r2_lance_path, indexed_by, indexed_at) VALUES
            ('rust', 'my-secret-app', '0.1.0', 'private', '{user_id}',
             '/tmp/fake/path.lance', '{user_id}', '2026-02-08T00:00:00Z')"
        ),
        (),
    )
    .await);

    // Owner query: should find it
    let mut rows = turso_op!(conn
        .query(
            &format!(
                "SELECT package FROM {table} WHERE
             visibility = 'public'
             OR (visibility = 'private' AND owner_id = ?1)"
            ),
            [user_id.as_str()],
        )
        .await);
    let row = turso_op!(rows.next().await)
        .expect("Owner should see private package");
    let pkg: String = row.get(0).unwrap();
    assert_eq!(pkg, "my-secret-app");
    eprintln!("  Owner ({user_id}) sees: {pkg}");

    // Non-owner query: should NOT find it
    let mut rows = turso_op!(conn
        .query(
            &format!(
                "SELECT package FROM {table} WHERE
             visibility = 'public'
             OR (visibility = 'private' AND owner_id = 'user_impostor')"
            ),
            (),
        )
        .await);
    let none = turso_op!(rows.next().await);
    assert!(none.is_none(), "Non-owner should NOT see private package");
    eprintln!("  Non-owner (user_impostor) sees: nothing");

    // Cleanup
    turso_op!(conn.execute(&format!("DROP TABLE {table}"), ()).await);
    eprintln!("  PASS: private code indexing — only owner can discover");
}

// ============================================================================
// Part L: Operational Concerns
// ============================================================================

/// L1: PRIMARY KEY prevents duplicate concurrent indexing.
///
/// The "unable to acquire shared lock on node (deletion must be in progress)"
/// error seen in parallel test runs is a **Turso infrastructure** issue, not
/// an application-level concurrency problem. It occurs when Turso nodes are
/// being provisioned/deleted (e.g., when L3 creates/deletes temp DBs).
///
/// This test validates that application-level concurrent writes are properly
/// handled by SQLite's UNIQUE constraint — which is a different mechanism
/// than Turso's node-level locking.
#[tokio::test(flavor = "multi_thread")]
async fn spike_concurrent_index_turso_lock() {
    let Some((url, token)) = turso_jwks_credentials().await else {
        eprintln!("SKIP: Turso/Clerk credentials not configured");
        return;
    };

    let db = match Builder::new_remote(url.clone(), token.clone())
        .build()
        .await
    {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Turso connection failed: {e}");
            return;
        }
    };
    let conn = db.connect().unwrap();

    let table = unique_table("idx_lock");
    conn.execute(&indexed_packages_ddl(&table), ())
        .await
        .unwrap();

    // First insert succeeds
    conn.execute(
        &format!(
            "INSERT INTO {table} (ecosystem, package, version, visibility,
             r2_lance_path, indexed_by, indexed_at) VALUES
            ('rust', 'tokio', '1.49.0', 'public',
             's3://zenith/lance/rust/tokio/1.49.0', 'user_first', '2026-02-08T00:00:00Z')"
        ),
        (),
    )
    .await
    .unwrap();
    eprintln!("  First INSERT succeeded");

    // Second insert with same PK should fail
    let result = conn
        .execute(
            &format!(
                "INSERT INTO {table} (ecosystem, package, version, visibility,
             r2_lance_path, indexed_by, indexed_at) VALUES
            ('rust', 'tokio', '1.49.0', 'public',
             's3://zenith/lance/rust/tokio/1.49.0', 'user_second', '2026-02-08T00:00:00Z')"
            ),
            (),
        )
        .await;

    assert!(
        result.is_err(),
        "Duplicate INSERT should fail with PK constraint"
    );
    let err_msg = format!("{}", result.unwrap_err());
    eprintln!("  Second INSERT correctly failed: {err_msg}");

    // Verify the check-then-skip pattern
    let mut rows = conn.query(
        &format!(
            "SELECT indexed_by FROM {table} WHERE ecosystem='rust' AND package='tokio' AND version='1.49.0'"
        ),
        (),
    ).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let indexed_by: String = row.get(0).unwrap();
    assert_eq!(indexed_by, "user_first", "First writer should win");

    // Now test actual concurrent writes from two separate connections
    let table2 = unique_table("idx_race");
    conn.execute(&indexed_packages_ddl(&table2), ())
        .await
        .unwrap();

    let url2 = url.clone();
    let token2 = token.clone();
    let table2_clone = table2.clone();

    // Spawn two tasks that try to insert the same package simultaneously
    let task_a = tokio::spawn({
        let url = url2.clone();
        let token = token2.clone();
        let table = table2_clone.clone();
        async move {
            let db = Builder::new_remote(url, token).build().await.unwrap();
            let conn = db.connect().unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            conn.execute(
                &format!(
                    "INSERT INTO {table} (ecosystem, package, version, visibility,
                     r2_lance_path, indexed_by, indexed_at) VALUES
                    ('rust', 'contested-pkg', '1.0.0', 'public',
                     's3://a', 'user_task_a', '2026-02-08T00:00:00Z')"
                ),
                (),
            )
            .await
        }
    });

    let task_b = tokio::spawn({
        let url = url2;
        let token = token2;
        let table = table2_clone;
        async move {
            let db = Builder::new_remote(url, token).build().await.unwrap();
            let conn = db.connect().unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            conn.execute(
                &format!(
                    "INSERT INTO {table} (ecosystem, package, version, visibility,
                     r2_lance_path, indexed_by, indexed_at) VALUES
                    ('rust', 'contested-pkg', '1.0.0', 'public',
                     's3://b', 'user_task_b', '2026-02-08T00:00:00Z')"
                ),
                (),
            )
            .await
        }
    });

    let (result_a, result_b) = tokio::join!(task_a, task_b);
    let a = result_a.unwrap();
    let b = result_b.unwrap();

    // Exactly one should succeed, the other should fail with UNIQUE constraint
    let a_ok = a.is_ok();
    let b_ok = b.is_ok();
    assert!(
        (a_ok && !b_ok) || (!a_ok && b_ok),
        "Exactly one concurrent INSERT should succeed: a={a_ok}, b={b_ok}"
    );

    let winner = if a_ok { "task_a" } else { "task_b" };
    eprintln!("  Concurrent race: {winner} won, other got UNIQUE constraint error");

    // Verify only one row exists
    let mut rows = conn
        .query(
            &format!("SELECT indexed_by FROM {table2} WHERE package = 'contested-pkg'"),
            (),
        )
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let who: String = row.get(0).unwrap();
    let second = rows.next().await.unwrap();
    assert!(second.is_none(), "Should be exactly 1 row");
    eprintln!("  Winner: {who}");

    // Cleanup
    conn.execute(&format!("DROP TABLE {table}"), ())
        .await
        .unwrap();
    conn.execute(&format!("DROP TABLE {table2}"), ())
        .await
        .unwrap();
    eprintln!(
        "  PASS: PRIMARY KEY prevents duplicate indexing — first writer wins, concurrent race resolved"
    );
}

/// L3: Two embedded replicas (different DBs) coexist in same process.
#[tokio::test(flavor = "multi_thread")]
async fn spike_two_turso_replicas_same_process() {
    load_env();
    let api_key = std::env::var("ZENITH_TURSO__PLATFORM_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("SKIP: ZENITH_TURSO__PLATFORM_API_KEY not set");
        return;
    }

    let Some((url_a, token_a)) = turso_jwks_credentials().await else {
        eprintln!("SKIP: Turso/Clerk credentials not configured");
        return;
    };

    // Create a temporary second DB
    let Some((url_b, db_b_name)) = create_temp_turso_db("spike20-l3").await else {
        eprintln!("SKIP: Failed to create temp Turso DB");
        return;
    };

    // Generate a Platform API token for DB B (Clerk JWT won't work for a new DB
    // unless JWKS is registered for it, so we use Platform API token instead)
    let db_b_name_for_token = db_b_name.clone();
    let Some(token_b) = turso_platform_token(&db_b_name_for_token).await else {
        eprintln!("SKIP: Failed to get token for temp DB");
        delete_turso_db(&db_b_name).await;
        return;
    };

    eprintln!("  DB A: {url_a}");
    eprintln!("  DB B: {url_b}");

    let tmp = TempDir::new().unwrap();

    // Open replica A
    let path_a = tmp
        .path()
        .join("replica_a.db")
        .to_string_lossy()
        .to_string();
    let replica_a = match Builder::new_remote_replica(path_a, url_a, token_a)
        .read_your_writes(true)
        .build()
        .await
    {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Replica A creation failed: {e}");
            delete_turso_db(&db_b_name).await;
            return;
        }
    };

    // Open replica B
    let path_b = tmp
        .path()
        .join("replica_b.db")
        .to_string_lossy()
        .to_string();
    let replica_b = match Builder::new_remote_replica(path_b, url_b, token_b)
        .read_your_writes(true)
        .build()
        .await
    {
        Ok(db) => db,
        Err(e) => {
            eprintln!("SKIP: Replica B creation failed: {e}");
            delete_turso_db(&db_b_name).await;
            return;
        }
    };

    // Sync both
    turso_op!(replica_a.sync().await);
    turso_op!(replica_b.sync().await);

    // Write to A
    let conn_a = replica_a.connect().unwrap();
    turso_op!(conn_a
        .execute(
            "CREATE TABLE IF NOT EXISTS test_a (id INTEGER PRIMARY KEY, val TEXT)",
            (),
        )
        .await
        .map(|_| ()));
    turso_op!(conn_a
        .execute("INSERT OR REPLACE INTO test_a VALUES (1, 'from_a')", ())
        .await
        .map(|_| ()));

    // Write to B
    let conn_b = replica_b.connect().unwrap();
    turso_op!(conn_b
        .execute(
            "CREATE TABLE IF NOT EXISTS test_b (id INTEGER PRIMARY KEY, val TEXT)",
            (),
        )
        .await
        .map(|_| ()));
    turso_op!(conn_b
        .execute("INSERT OR REPLACE INTO test_b VALUES (1, 'from_b')", ())
        .await
        .map(|_| ()));

    // Sync both
    turso_op!(replica_a.sync().await);
    turso_op!(replica_b.sync().await);

    // Query A — should NOT see test_b
    let has_test_b = turso_op!(async {
        let mut rows = conn_a
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='test_b'",
                (),
            )
            .await?;
        rows.next().await.map(|row| row.is_some())
    }
    .await);
    assert!(!has_test_b, "DB A should NOT have test_b table");

    // Query B — should NOT see test_a
    let has_test_a = turso_op!(async {
        let mut rows = conn_b
            .query(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='test_a'",
                (),
            )
            .await?;
        rows.next().await.map(|row| row.is_some())
    }
    .await);
    assert!(!has_test_a, "DB B should NOT have test_a table");

    // Query A — should see its own data
    let val_a: String = turso_op!(async {
        let mut rows = conn_a.query("SELECT val FROM test_a WHERE id = 1", ()).await?;
        let Some(row) = rows.next().await? else {
            return Err(libsql::Error::SqliteFailure(1, "missing row for test_a".to_string()));
        };
        row.get(0)
    }
    .await);
    assert_eq!(val_a, "from_a");

    // Query B — should see its own data
    let val_b: String = turso_op!(async {
        let mut rows = conn_b.query("SELECT val FROM test_b WHERE id = 1", ()).await?;
        let Some(row) = rows.next().await? else {
            return Err(libsql::Error::SqliteFailure(1, "missing row for test_b".to_string()));
        };
        row.get(0)
    }
    .await);
    assert_eq!(val_b, "from_b");

    eprintln!("  Both replicas coexist — isolated data, no interference");

    drop(conn_b);
    drop(replica_b);

    // Cleanup
    turso_op!(conn_a.execute("DROP TABLE IF EXISTS test_a", ()).await.map(|_| ()));
    turso_op!(replica_a.sync().await);
    delete_turso_db(&db_b_name).await;

    eprintln!("  PASS: two Turso replicas in same process — no interference");
}
