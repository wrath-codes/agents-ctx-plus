//! Integration tests for TOML configuration loading.
//!
//! Uses figment::Jail for safe, sandboxed env var manipulation.
//! Pattern from: aether `aether-config/tests/toml_loading.rs`.

use figment::{
    Figment, Jail,
    providers::{Env, Format, Serialized, Toml},
};
use zen_config::ZenConfig;

#[test]
fn loads_turso_config_from_toml() {
    Jail::expect_with(|jail| {
        jail.create_file(
            "config.toml",
            r#"
[turso]
url = "libsql://test.turso.io"
auth_token = "turso-token"
platform_api_key = "plat-key"
org_slug = "my-org"
sync_interval_secs = 120
read_your_writes = false
local_replica_path = "./replica.db"
"#,
        )?;

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Toml::file("config.toml"))
            .extract()?;

        assert_eq!(config.turso.url, "libsql://test.turso.io");
        assert_eq!(config.turso.auth_token, "turso-token");
        assert_eq!(config.turso.platform_api_key, "plat-key");
        assert_eq!(config.turso.org_slug, "my-org");
        assert_eq!(config.turso.sync_interval_secs, 120);
        assert!(!config.turso.read_your_writes);
        assert_eq!(config.turso.local_replica_path, "./replica.db");
        assert!(config.turso.is_configured());
        assert!(config.turso.can_mint_tokens());
        Ok(())
    });
}

#[test]
fn loads_r2_config_from_toml() {
    Jail::expect_with(|jail| {
        jail.create_file(
            "config.toml",
            r#"
[r2]
account_id = "toml-account"
access_key_id = "toml-key"
secret_access_key = "toml-secret"
bucket_name = "toml-bucket"
endpoint = "http://localhost:9000"
"#,
        )?;

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Toml::file("config.toml"))
            .extract()?;

        assert_eq!(config.r2.account_id, "toml-account");
        assert_eq!(config.r2.access_key_id, "toml-key");
        assert_eq!(config.r2.secret_access_key, "toml-secret");
        assert_eq!(config.r2.bucket_name, "toml-bucket");
        assert_eq!(config.r2.endpoint, "http://localhost:9000");
        assert!(config.r2.is_configured());
        Ok(())
    });
}

#[test]
fn loads_motherduck_config_from_toml() {
    Jail::expect_with(|jail| {
        jail.create_file(
            "config.toml",
            r#"
[motherduck]
access_token = "md-token-123"
db_name = "custom_db"
"#,
        )?;

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Toml::file("config.toml"))
            .extract()?;

        assert_eq!(config.motherduck.access_token, "md-token-123");
        assert_eq!(config.motherduck.db_name, "custom_db");
        assert!(config.motherduck.is_configured());
        Ok(())
    });
}

#[test]
fn loads_clerk_config_from_toml() {
    Jail::expect_with(|jail| {
        jail.create_file(
            "config.toml",
            r#"
[clerk]
publishable_key = "pk_test_123"
secret_key = "sk_test_456"
jwks_url = "https://clerk.dev/.well-known/jwks.json"
backend_url = "https://api.clerk.dev"
frontend_url = "https://app.clerk.dev"
"#,
        )?;

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Toml::file("config.toml"))
            .extract()?;

        assert_eq!(config.clerk.publishable_key, "pk_test_123");
        assert_eq!(config.clerk.secret_key, "sk_test_456");
        assert_eq!(
            config.clerk.jwks_url,
            "https://clerk.dev/.well-known/jwks.json"
        );
        assert!(config.clerk.is_configured());
        Ok(())
    });
}

#[test]
fn loads_axiom_config_from_toml() {
    Jail::expect_with(|jail| {
        jail.create_file(
            "config.toml",
            r#"
[axiom]
token = "xaat-test-token"
dataset = "aether-traces"
endpoint = "https://custom-axiom.co"
"#,
        )?;

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Toml::file("config.toml"))
            .extract()?;

        assert_eq!(config.axiom.token, "xaat-test-token");
        assert_eq!(config.axiom.dataset, "aether-traces");
        assert_eq!(config.axiom.endpoint, "https://custom-axiom.co");
        assert!(config.axiom.is_configured());
        Ok(())
    });
}

#[test]
fn loads_full_config_from_toml() {
    Jail::expect_with(|jail| {
        jail.create_file(
            "config.toml",
            r#"
[turso]
url = "libsql://db.turso.io"
auth_token = "token"

[r2]
account_id = "acc"
access_key_id = "key"
secret_access_key = "secret"
bucket_name = "bucket"

[motherduck]
access_token = "md-token"
db_name = "zenith"

[clerk]
publishable_key = "pk"
secret_key = "sk"

[axiom]
token = "xaat-token"
dataset = "traces"

[general]
auto_commit = true
default_ecosystem = "rust"
default_limit = 50
"#,
        )?;

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Toml::file("config.toml"))
            .extract()?;

        assert!(config.turso.is_configured());
        assert!(config.r2.is_configured());
        assert!(config.motherduck.is_configured());
        assert!(config.clerk.is_configured());
        assert!(config.axiom.is_configured());
        assert!(config.general.auto_commit);
        assert_eq!(config.general.default_ecosystem, "rust");
        assert_eq!(config.general.default_limit, 50);
        Ok(())
    });
}

#[test]
fn env_var_overrides_toml() {
    Jail::expect_with(|jail| {
        jail.set_env("ZENITH_TURSO__URL", "libsql://from-env.turso.io");

        jail.create_file(
            "config.toml",
            r#"
[turso]
url = "libsql://from-toml.turso.io"
auth_token = "toml-token"
"#,
        )?;

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("ZENITH_").split("__"))
            .extract()?;

        // Env should win over TOML
        assert_eq!(config.turso.url, "libsql://from-env.turso.io");
        // TOML value not overridden by env should remain
        assert_eq!(config.turso.auth_token, "toml-token");
        Ok(())
    });
}

#[test]
fn env_var_overrides_default() {
    Jail::expect_with(|jail| {
        jail.set_env("ZENITH_R2__ACCOUNT_ID", "env-account-id");

        // No TOML file -- just defaults + env
        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Env::prefixed("ZENITH_").split("__"))
            .extract()?;

        assert_eq!(config.r2.account_id, "env-account-id");
        Ok(())
    });
}

/// Documents the figment gotcha: typo'd env var keys are silently ignored.
/// The value stays at its default because figment doesn't know "urll" should be "url".
#[test]
fn typo_env_var_silently_ignored() {
    Jail::expect_with(|jail| {
        jail.set_env("ZENITH_TURSO__URLL", "libsql://typo.turso.io");

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Env::prefixed("ZENITH_").split("__"))
            .extract()?;

        // "urll" is not a known field -- silently ignored, url stays at default (empty)
        assert!(
            config.turso.url.is_empty(),
            "typo'd env var should be silently ignored by figment"
        );
        Ok(())
    });
}

/// Verify that figment's Env provider correctly maps nested ZENITH_* vars
/// through the full provider chain (defaults -> env).
#[test]
fn full_env_provider_chain() {
    Jail::expect_with(|jail| {
        jail.set_env("ZENITH_TURSO__URL", "libsql://jail.turso.io");
        jail.set_env("ZENITH_TURSO__AUTH_TOKEN", "jail-token");
        jail.set_env("ZENITH_TURSO__ORG_SLUG", "jail-org");
        jail.set_env("ZENITH_R2__ACCOUNT_ID", "jail-account");
        jail.set_env("ZENITH_R2__ACCESS_KEY_ID", "jail-key");
        jail.set_env("ZENITH_R2__SECRET_ACCESS_KEY", "jail-secret");
        jail.set_env("ZENITH_MOTHERDUCK__ACCESS_TOKEN", "jail-md-token");
        jail.set_env("ZENITH_CLERK__PUBLISHABLE_KEY", "pk_jail");
        jail.set_env("ZENITH_CLERK__SECRET_KEY", "sk_jail");
        jail.set_env("ZENITH_AXIOM__TOKEN", "xaat-jail");
        jail.set_env("ZENITH_AXIOM__DATASET", "jail-traces");
        jail.set_env("ZENITH_GENERAL__DEFAULT_LIMIT", "42");

        let config: ZenConfig = Figment::from(Serialized::defaults(ZenConfig::default()))
            .merge(Env::prefixed("ZENITH_").split("__"))
            .extract()?;

        assert_eq!(config.turso.url, "libsql://jail.turso.io");
        assert_eq!(config.turso.auth_token, "jail-token");
        assert_eq!(config.turso.org_slug, "jail-org");
        assert!(config.turso.is_configured());

        assert_eq!(config.r2.account_id, "jail-account");
        assert_eq!(config.r2.access_key_id, "jail-key");
        assert_eq!(config.r2.secret_access_key, "jail-secret");

        assert_eq!(config.motherduck.access_token, "jail-md-token");
        assert!(config.motherduck.is_configured());

        assert_eq!(config.clerk.publishable_key, "pk_jail");
        assert_eq!(config.clerk.secret_key, "sk_jail");
        assert!(config.clerk.is_configured());

        assert_eq!(config.axiom.token, "xaat-jail");
        assert_eq!(config.axiom.dataset, "jail-traces");
        assert!(config.axiom.is_configured());

        assert_eq!(config.general.default_limit, 42);
        Ok(())
    });
}
