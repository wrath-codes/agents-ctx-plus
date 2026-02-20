//! # zen-secrets
//!
//! External secret provider integrations for Zenith.

use infisical::{AuthMethod, Client, secrets::ListSecretsRequest};
use thiserror::Error;

const ENV_BACKEND: &str = "ZENITH_SECRETS__BACKEND";
const ENV_INFISICAL_BASE_URL: &str = "ZENITH_INFISICAL__BASE_URL";
const ENV_INFISICAL_CLIENT_ID: &str = "ZENITH_INFISICAL__CLIENT_ID";
const ENV_INFISICAL_CLIENT_SECRET: &str = "ZENITH_INFISICAL__CLIENT_SECRET";
const ENV_INFISICAL_PROJECT_ID: &str = "ZENITH_INFISICAL__PROJECT_ID";
const ENV_INFISICAL_ENVIRONMENT: &str = "ZENITH_INFISICAL__ENVIRONMENT";
const ENV_INFISICAL_PATH: &str = "ZENITH_INFISICAL__PATH";

/// Result of resolving external secrets.
#[derive(Debug, Clone)]
pub enum SecretOverrides {
    Disabled,
    Values(Vec<(String, String)>),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Backend {
    None,
    Infisical,
}

impl Backend {
    fn from_env() -> Result<Self, SecretError> {
        let raw = std::env::var(ENV_BACKEND).unwrap_or_default();
        let normalized = raw.trim().to_ascii_lowercase();

        match normalized.as_str() {
            "" | "none" | "off" | "disabled" => Ok(Self::None),
            "infisical" => Ok(Self::Infisical),
            value => Err(SecretError::UnsupportedBackend(value.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
struct InfisicalSettings {
    base_url: String,
    client_id: String,
    client_secret: String,
    project_id: String,
    environment: String,
    path: String,
}

impl InfisicalSettings {
    fn from_env() -> Result<Self, SecretError> {
        Ok(Self {
            base_url: std::env::var(ENV_INFISICAL_BASE_URL)
                .unwrap_or_else(|_| "https://app.infisical.com".to_string()),
            client_id: required_env(ENV_INFISICAL_CLIENT_ID)?,
            client_secret: required_env(ENV_INFISICAL_CLIENT_SECRET)?,
            project_id: required_env(ENV_INFISICAL_PROJECT_ID)?,
            environment: required_env(ENV_INFISICAL_ENVIRONMENT)?,
            path: std::env::var(ENV_INFISICAL_PATH).unwrap_or_else(|_| "/".to_string()),
        })
    }
}

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("unsupported secrets backend '{0}'")]
    UnsupportedBackend(String),
    #[error("required environment variable '{name}' is missing")]
    MissingEnvVar { name: &'static str },
    #[error("infisical error: {0}")]
    Infisical(#[from] infisical::InfisicalError),
}

fn required_env(name: &'static str) -> Result<String, SecretError> {
    std::env::var(name).map_err(|_| SecretError::MissingEnvVar { name })
}

/// Load secret key/value overrides from the configured external backend.
///
/// Expected naming convention is exact config keys (e.g., `ZENITH_CLERK__SECRET_KEY`).
pub async fn load_env_overrides() -> Result<SecretOverrides, SecretError> {
    match Backend::from_env()? {
        Backend::None => Ok(SecretOverrides::Disabled),
        Backend::Infisical => {
            let settings = InfisicalSettings::from_env()?;
            let values = load_from_infisical(&settings).await?;
            Ok(SecretOverrides::Values(values))
        }
    }
}

async fn load_from_infisical(
    settings: &InfisicalSettings,
) -> Result<Vec<(String, String)>, SecretError> {
    let mut client = Client::builder()
        .base_url(&settings.base_url)
        .build()
        .await?;

    client
        .login(AuthMethod::new_universal_auth(
            &settings.client_id,
            &settings.client_secret,
        ))
        .await?;

    let request = ListSecretsRequest::builder(&settings.project_id, &settings.environment)
        .path(&settings.path)
        .recursive(true)
        .expand_secret_references(true)
        .build();

    let mut values = client
        .secrets()
        .list(request)
        .await?
        .into_iter()
        .filter(|secret| secret.secret_key.starts_with("ZENITH_"))
        .map(|secret| (secret.secret_key, secret.secret_value))
        .collect::<Vec<_>>();

    values.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::Backend;

    #[test]
    fn backend_defaults_to_none_when_missing() {
        figment::Jail::expect_with(|_jail| {
            let backend = Backend::from_env().expect("backend should parse");
            assert_eq!(backend, Backend::None);
            Ok(())
        });
    }

    #[test]
    fn backend_parses_infisical() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("ZENITH_SECRETS__BACKEND", "infisical");
            let backend = Backend::from_env().expect("backend should parse");
            assert_eq!(backend, Backend::Infisical);
            Ok(())
        });
    }
}
