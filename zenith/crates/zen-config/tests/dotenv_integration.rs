//! Integration tests that prove real values from `.env` flow through figment correctly.
//!
//! These tests load the workspace `.env` file via `dotenvy`, then use `ZenConfig::load()`
//! to extract the config through figment's full provider chain. They verify that real
//! credentials are accessible and produce valid outputs for downstream consumers
//! (zen-db, zen-lake).
//!
//! Tests skip gracefully when `.env` is not present or credentials are missing.

use zen_config::ZenConfig;

/// Load .env from the workspace root.
fn load_env() {
    let workspace_env = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(".env"));

    if let Some(env_path) = workspace_env {
        let _ = dotenvy::from_path(&env_path);
    }
}

// ---------------------------------------------------------------------------
// Turso
// ---------------------------------------------------------------------------

#[test]
fn dotenv_loads_real_turso_config() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    if !config.turso.can_mint_tokens() {
        eprintln!("SKIP: Turso not configured for token minting");
        return;
    }

    // URL should be a real libsql:// URL
    assert!(
        config.turso.url.starts_with("libsql://"),
        "turso.url should start with libsql://, got: {}",
        config.turso.url
    );

    // Org slug should be non-empty
    assert!(
        !config.turso.org_slug.is_empty(),
        "turso.org_slug should be set"
    );

    // Platform API key should be non-empty (JWT)
    assert!(
        config.turso.platform_api_key.starts_with("eyJ"),
        "turso.platform_api_key should be a JWT, got: {}...",
        &config.turso.platform_api_key[..20.min(config.turso.platform_api_key.len())]
    );

    // db_name() extraction should work with real URL + org_slug
    let db_name = config.turso.db_name();
    assert!(
        db_name.is_some(),
        "db_name() should extract from real URL: {}",
        config.turso.url
    );
    eprintln!("OK: turso.url={}", config.turso.url);
    eprintln!("OK: turso.org_slug={}", config.turso.org_slug);
    eprintln!("OK: turso.db_name()={}", db_name.unwrap());
}

// ---------------------------------------------------------------------------
// R2
// ---------------------------------------------------------------------------

#[test]
fn dotenv_loads_real_r2_config() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    if !config.r2.is_configured() {
        eprintln!("SKIP: R2 not configured");
        return;
    }

    // Account ID should look like a Cloudflare account ID (32-char hex)
    assert!(
        config.r2.account_id.len() == 32,
        "r2.account_id should be 32 chars, got {}",
        config.r2.account_id.len()
    );

    // endpoint_url() should produce a valid URL
    let endpoint = config.r2.endpoint_url();
    assert!(
        endpoint.starts_with("https://"),
        "r2.endpoint_url() should start with https://, got: {}",
        endpoint
    );

    // create_secret_sql() should produce valid-looking SQL with real credentials
    let sql = config.r2.create_secret_sql("r2_test");
    assert!(sql.contains("TYPE s3"));
    assert!(sql.contains(&config.r2.access_key_id));
    assert!(sql.contains(&config.r2.secret_access_key));
    assert!(sql.contains("r2.cloudflarestorage.com"));

    eprintln!("OK: r2.account_id={}...", &config.r2.account_id[..8]);
    eprintln!("OK: r2.endpoint_url()={}", endpoint);
    eprintln!("OK: r2.create_secret_sql() is {} bytes", sql.len());
}

// ---------------------------------------------------------------------------
// MotherDuck
// ---------------------------------------------------------------------------

#[test]
fn dotenv_loads_real_motherduck_config() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    if !config.motherduck.is_configured() {
        eprintln!("SKIP: MotherDuck not configured");
        return;
    }

    // Token should be a JWT
    assert!(
        config.motherduck.access_token.starts_with("eyJ"),
        "motherduck.access_token should be a JWT"
    );

    // connection_string() should produce a valid md: URL
    let conn_str = config.motherduck.connection_string();
    assert!(
        conn_str.starts_with("md:"),
        "connection_string() should start with md:, got: {}",
        conn_str
    );
    assert!(
        conn_str.contains("motherduck_token="),
        "connection_string() should contain motherduck_token="
    );

    eprintln!("OK: motherduck.db_name={}", config.motherduck.db_name);
    eprintln!(
        "OK: motherduck.connection_string()=md:{}?motherduck_token=eyJ...",
        config.motherduck.db_name
    );
}

// ---------------------------------------------------------------------------
// Clerk
// ---------------------------------------------------------------------------

#[test]
fn dotenv_loads_real_clerk_config() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    if !config.clerk.is_configured() {
        eprintln!("SKIP: Clerk not configured");
        return;
    }

    assert!(
        config.clerk.publishable_key.starts_with("pk_"),
        "clerk.publishable_key should start with pk_, got: {}",
        config.clerk.publishable_key
    );

    assert!(
        !config.clerk.jwks_url.is_empty(),
        "clerk.jwks_url should be set"
    );

    eprintln!(
        "OK: clerk.publishable_key={}...",
        &config.clerk.publishable_key[..20.min(config.clerk.publishable_key.len())]
    );
    eprintln!("OK: clerk.jwks_url={}", config.clerk.jwks_url);
}

// ---------------------------------------------------------------------------
// Axiom
// ---------------------------------------------------------------------------

#[test]
fn dotenv_loads_real_axiom_config() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    if !config.axiom.is_configured() {
        eprintln!("SKIP: Axiom not configured");
        return;
    }

    assert!(
        config.axiom.is_valid_token(),
        "axiom.token should start with xaat-, got: {}...",
        &config.axiom.token[..10.min(config.axiom.token.len())]
    );

    assert!(
        !config.axiom.dataset.is_empty(),
        "axiom.dataset should be set"
    );

    assert!(
        config.axiom.endpoint.starts_with("https://"),
        "axiom.endpoint should be a URL, got: {}",
        config.axiom.endpoint
    );

    eprintln!("OK: axiom.dataset={}", config.axiom.dataset);
    eprintln!("OK: axiom.endpoint={}", config.axiom.endpoint);
}

// ---------------------------------------------------------------------------
// Full config: all sections loaded from .env
// ---------------------------------------------------------------------------

#[test]
fn dotenv_loads_all_configured_sections() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    let mut configured = Vec::new();
    let mut unconfigured = Vec::new();

    if config.turso.can_mint_tokens() {
        configured.push("turso");
    } else {
        unconfigured.push("turso");
    }
    if config.r2.is_configured() {
        configured.push("r2");
    } else {
        unconfigured.push("r2");
    }
    if config.motherduck.is_configured() {
        configured.push("motherduck");
    } else {
        unconfigured.push("motherduck");
    }
    if config.clerk.is_configured() {
        configured.push("clerk");
    } else {
        unconfigured.push("clerk");
    }
    if config.axiom.is_configured() {
        configured.push("axiom");
    } else {
        unconfigured.push("axiom");
    }

    eprintln!("Configured: {:?}", configured);
    if !unconfigured.is_empty() {
        eprintln!("Not configured: {:?}", unconfigured);
    }

    // With a full .env, all sections should be configured
    // This test documents the expected state rather than hard-failing
    if configured.len() < 3 {
        eprintln!(
            "WARNING: Only {}/5 sections configured — check .env file",
            configured.len()
        );
    }
}

// ---------------------------------------------------------------------------
// Spike compatibility: values match what existing spikes read
// ---------------------------------------------------------------------------

/// Prove that the values figment extracts from ZENITH_TURSO__* env vars match
/// what spike_libsql_sync.rs reads via std::env::var("ZENITH_TURSO__URL").
#[test]
fn config_matches_spike_turso_env_vars() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    // These are the exact env vars that spike_libsql_sync.rs reads
    let spike_url = std::env::var("ZENITH_TURSO__URL").unwrap_or_default();
    let spike_api_key = std::env::var("ZENITH_TURSO__PLATFORM_API_KEY").unwrap_or_default();
    let spike_org = std::env::var("ZENITH_TURSO__ORG_SLUG").unwrap_or_default();

    assert_eq!(
        config.turso.url, spike_url,
        "figment turso.url should match env var ZENITH_TURSO__URL"
    );
    assert_eq!(
        config.turso.platform_api_key, spike_api_key,
        "figment turso.platform_api_key should match env var ZENITH_TURSO__PLATFORM_API_KEY"
    );
    assert_eq!(
        config.turso.org_slug, spike_org,
        "figment turso.org_slug should match env var ZENITH_TURSO__ORG_SLUG"
    );
}

/// Prove that the values figment extracts from ZENITH_R2__* env vars match
/// what spike_duckdb_vss.rs reads via std::env::var("ZENITH_R2__ACCOUNT_ID").
#[test]
fn config_matches_spike_r2_env_vars() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    let spike_account = std::env::var("ZENITH_R2__ACCOUNT_ID").unwrap_or_default();
    let spike_key = std::env::var("ZENITH_R2__ACCESS_KEY_ID").unwrap_or_default();
    let spike_secret = std::env::var("ZENITH_R2__SECRET_ACCESS_KEY").unwrap_or_default();
    let spike_bucket = std::env::var("ZENITH_R2__BUCKET_NAME").unwrap_or_default();

    assert_eq!(config.r2.account_id, spike_account);
    assert_eq!(config.r2.access_key_id, spike_key);
    assert_eq!(config.r2.secret_access_key, spike_secret);
    assert_eq!(config.r2.bucket_name, spike_bucket);
}

/// Prove that the values figment extracts from ZENITH_MOTHERDUCK__* env vars match
/// what spike_duckdb_vss.rs reads via std::env::var("ZENITH_MOTHERDUCK__ACCESS_TOKEN").
#[test]
fn config_matches_spike_motherduck_env_vars() {
    load_env();
    let config = ZenConfig::load().expect("config loads");

    let spike_token = std::env::var("ZENITH_MOTHERDUCK__ACCESS_TOKEN").unwrap_or_default();

    assert_eq!(config.motherduck.access_token, spike_token);
}
