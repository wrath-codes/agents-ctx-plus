use zen_config::ZenConfig;

/// Emit warnings for likely mistyped env var keys that silently fell back to defaults.
pub fn warn_unconfigured(config: &ZenConfig) {
    if config.turso.url.is_empty() && has_env_prefix("ZENITH_TURSO") {
        tracing::warn!(
            "Turso config appears default while ZENITH_TURSO* env vars exist. Use double underscores (example: ZENITH_TURSO__URL)."
        );
    }

    if config.r2.account_id.is_empty() && has_env_prefix("ZENITH_R2") {
        tracing::warn!(
            "R2 config appears default while ZENITH_R2* env vars exist. Use double underscores (example: ZENITH_R2__ACCOUNT_ID)."
        );
    }

    if config.motherduck.access_token.is_empty() && has_env_prefix("ZENITH_MOTHERDUCK") {
        tracing::warn!(
            "MotherDuck config appears default while ZENITH_MOTHERDUCK* env vars exist. Use double underscores (example: ZENITH_MOTHERDUCK__ACCESS_TOKEN)."
        );
    }
}

fn has_env_prefix(prefix: &str) -> bool {
    std::env::vars().any(|(key, _)| key.starts_with(prefix))
}
