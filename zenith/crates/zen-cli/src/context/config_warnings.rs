use zen_config::ZenConfig;

/// Emit warnings for likely mistyped env var keys that silently fell back to defaults.
pub fn warn_unconfigured(config: &ZenConfig) {
    for warning in collect_unconfigured_warnings(config, std::env::vars()) {
        tracing::warn!("{warning}");
    }
}

fn collect_unconfigured_warnings<I>(config: &ZenConfig, env: I) -> Vec<String>
where
    I: IntoIterator<Item = (String, String)>,
{
    let env_keys = env.into_iter().map(|(key, _)| key).collect::<Vec<_>>();

    let mut warnings = Vec::new();

    if !config.turso.is_configured() && has_env_prefix(&env_keys, "ZENITH_TURSO") {
        warnings.push(
            "Turso config appears default while ZENITH_TURSO* env vars exist. Use double underscores (example: ZENITH_TURSO__URL)."
                .to_string(),
        );
    }

    if !config.r2.is_configured() && has_env_prefix(&env_keys, "ZENITH_R2") {
        warnings.push(
            "R2 config appears default while ZENITH_R2* env vars exist. Use double underscores (example: ZENITH_R2__ACCOUNT_ID)."
                .to_string(),
        );
    }

    if !config.motherduck.is_configured() && has_env_prefix(&env_keys, "ZENITH_MOTHERDUCK") {
        warnings.push(
            "MotherDuck config appears default while ZENITH_MOTHERDUCK* env vars exist. Use double underscores (example: ZENITH_MOTHERDUCK__ACCESS_TOKEN)."
                .to_string(),
        );
    }

    if !config.clerk.is_configured() && has_env_prefix(&env_keys, "ZENITH_CLERK") {
        warnings.push(
            "Clerk config appears default while ZENITH_CLERK* env vars exist. Use double underscores (example: ZENITH_CLERK__PUBLISHABLE_KEY)."
                .to_string(),
        );
    }

    if !config.axiom.is_configured() && has_env_prefix(&env_keys, "ZENITH_AXIOM") {
        warnings.push(
            "Axiom config appears default while ZENITH_AXIOM* env vars exist. Use double underscores (example: ZENITH_AXIOM__TOKEN)."
                .to_string(),
        );
    }

    warnings
}

fn has_env_prefix(keys: &[String], prefix: &str) -> bool {
    keys.iter().any(|key| key.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use zen_config::ZenConfig;

    use super::collect_unconfigured_warnings;

    #[test]
    fn warns_for_unconfigured_sections_with_env_prefixes() {
        let config = ZenConfig::default();
        let warnings = collect_unconfigured_warnings(
            &config,
            vec![
                ("ZENITH_TURSO__URL".to_string(), "libsql://demo".to_string()),
                ("ZENITH_R2__ACCOUNT_ID".to_string(), "abc".to_string()),
                (
                    "ZENITH_MOTHERDUCK__ACCESS_TOKEN".to_string(),
                    "token".to_string(),
                ),
                (
                    "ZENITH_CLERK__PUBLISHABLE_KEY".to_string(),
                    "pk_test".to_string(),
                ),
                ("ZENITH_AXIOM__TOKEN".to_string(), "xaat-123".to_string()),
            ],
        );

        assert_eq!(warnings.len(), 5);
    }

    #[test]
    fn does_not_warn_when_sections_are_configured() {
        let config = ZenConfig {
            turso: zen_config::TursoConfig {
                url: "libsql://demo".to_string(),
                auth_token: "token".to_string(),
                ..Default::default()
            },
            r2: zen_config::R2Config {
                account_id: "acc".to_string(),
                access_key_id: "key".to_string(),
                secret_access_key: "secret".to_string(),
                ..Default::default()
            },
            motherduck: zen_config::MotherDuckConfig {
                access_token: "token".to_string(),
                ..Default::default()
            },
            clerk: zen_config::ClerkConfig {
                publishable_key: "pk".to_string(),
                secret_key: "sk".to_string(),
                ..Default::default()
            },
            axiom: zen_config::AxiomConfig {
                token: "xaat-123".to_string(),
                dataset: "dataset".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let warnings = collect_unconfigured_warnings(
            &config,
            vec![
                ("ZENITH_TURSO__URL".to_string(), "libsql://demo".to_string()),
                ("ZENITH_R2__ACCOUNT_ID".to_string(), "acc".to_string()),
                (
                    "ZENITH_MOTHERDUCK__ACCESS_TOKEN".to_string(),
                    "token".to_string(),
                ),
                (
                    "ZENITH_CLERK__PUBLISHABLE_KEY".to_string(),
                    "pk".to_string(),
                ),
                ("ZENITH_AXIOM__TOKEN".to_string(), "xaat-123".to_string()),
            ],
        );

        assert!(warnings.is_empty());
    }
}
