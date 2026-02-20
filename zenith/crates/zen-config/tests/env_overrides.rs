use figment::Jail;
use zen_config::ZenConfig;

#[test]
fn external_overrides_fill_config_values() {
    Jail::expect_with(|_jail| {
        let overrides = vec![(
            "ZENITH_CLERK__SECRET_KEY".to_string(),
            "sk_from_external".to_string(),
        )];

        let config = ZenConfig::load_with_env_overrides(&overrides).expect("config loads");
        assert_eq!(config.clerk.secret_key, "sk_from_external");
        Ok(())
    });
}

#[test]
fn process_env_beats_external_overrides() {
    Jail::expect_with(|jail| {
        jail.set_env("ZENITH_CLERK__SECRET_KEY", "sk_from_env");
        let overrides = vec![(
            "ZENITH_CLERK__SECRET_KEY".to_string(),
            "sk_from_external".to_string(),
        )];

        let config = ZenConfig::load_with_env_overrides(&overrides).expect("config loads");
        assert_eq!(config.clerk.secret_key, "sk_from_env");
        Ok(())
    });
}
