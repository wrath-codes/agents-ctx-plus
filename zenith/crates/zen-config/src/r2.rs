//! Cloudflare R2 configuration.

use serde::{Deserialize, Serialize};

/// Default bucket name.
fn default_bucket_name() -> String {
    String::from("zenith")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct R2Config {
    /// Cloudflare account ID.
    #[serde(default)]
    pub account_id: String,

    /// R2 access key ID.
    #[serde(default)]
    pub access_key_id: String,

    /// R2 secret access key.
    #[serde(default)]
    pub secret_access_key: String,

    /// R2 bucket name.
    #[serde(default = "default_bucket_name")]
    pub bucket_name: String,

    /// Custom endpoint URL. If empty, built from `account_id`.
    #[serde(default)]
    pub endpoint: String,
}

impl Default for R2Config {
    fn default() -> Self {
        Self {
            account_id: String::new(),
            access_key_id: String::new(),
            secret_access_key: String::new(),
            bucket_name: default_bucket_name(),
            endpoint: String::new(),
        }
    }
}

impl R2Config {
    /// Check if the R2 config has the minimum required fields.
    pub fn is_configured(&self) -> bool {
        !self.account_id.is_empty()
            && !self.access_key_id.is_empty()
            && !self.secret_access_key.is_empty()
            && !self.bucket_name.is_empty()
    }

    /// Build the R2 endpoint URL.
    ///
    /// Returns the custom `endpoint` if set, otherwise builds from `account_id`.
    pub fn endpoint_url(&self) -> String {
        if self.endpoint.is_empty() {
            format!("https://{}.r2.cloudflarestorage.com", self.account_id)
        } else {
            self.endpoint.clone()
        }
    }

    /// Generate the DuckDB SQL to create an R2 secret.
    ///
    /// This produces a `CREATE SECRET` statement that DuckDB uses to access R2
    /// via the httpfs extension. The `secret_name` parameter allows creating
    /// distinct secrets (e.g., `r2_zenith`, `r2_spike`).
    pub fn create_secret_sql(&self, secret_name: &str) -> String {
        format!(
            "CREATE SECRET IF NOT EXISTS {secret_name} (
    TYPE s3,
    KEY_ID '{key_id}',
    SECRET '{secret}',
    ENDPOINT '{endpoint}',
    URL_STYLE 'path'
)",
            secret_name = secret_name,
            key_id = self.access_key_id,
            secret = self.secret_access_key,
            endpoint = self.r2_endpoint(),
        )
    }

    /// Generate the DuckDB SQL to create an R2 secret stored in MotherDuck.
    ///
    /// When using MotherDuck as the catalog, secrets must be created `IN MOTHERDUCK`
    /// so they persist across sessions.
    pub fn create_secret_sql_motherduck(&self, secret_name: &str) -> String {
        format!(
            "CREATE SECRET IF NOT EXISTS {secret_name} IN MOTHERDUCK (
    TYPE s3,
    KEY_ID '{key_id}',
    SECRET '{secret}',
    ENDPOINT '{endpoint}',
    URL_STYLE 'path'
)",
            secret_name = secret_name,
            key_id = self.access_key_id,
            secret = self.secret_access_key,
            endpoint = self.r2_endpoint(),
        )
    }

    /// The R2-specific endpoint (without `https://` prefix).
    ///
    /// DuckDB `CREATE SECRET` expects just `account_id.r2.cloudflarestorage.com`.
    fn r2_endpoint(&self) -> String {
        format!("{}.r2.cloudflarestorage.com", self.account_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_configured() {
        let config = R2Config::default();
        assert!(!config.is_configured());
        assert_eq!(config.bucket_name, "zenith");
    }

    #[test]
    fn configured_when_all_required_fields_set() {
        let config = R2Config {
            account_id: "abc123".into(),
            access_key_id: "key".into(),
            secret_access_key: "secret".into(),
            bucket_name: "bucket".into(),
            endpoint: String::new(),
        };
        assert!(config.is_configured());
    }

    #[test]
    fn not_configured_when_missing_field() {
        let config = R2Config {
            account_id: "abc123".into(),
            access_key_id: String::new(), // missing
            secret_access_key: "secret".into(),
            bucket_name: "bucket".into(),
            endpoint: String::new(),
        };
        assert!(!config.is_configured());
    }

    #[test]
    fn endpoint_url_built_from_account_id() {
        let config = R2Config {
            account_id: "abc123".into(),
            ..Default::default()
        };
        assert_eq!(
            config.endpoint_url(),
            "https://abc123.r2.cloudflarestorage.com"
        );
    }

    #[test]
    fn custom_endpoint_used_when_set() {
        let config = R2Config {
            endpoint: "http://localhost:9000".into(),
            ..Default::default()
        };
        assert_eq!(config.endpoint_url(), "http://localhost:9000");
    }

    #[test]
    fn create_secret_sql_contains_credentials() {
        let config = R2Config {
            account_id: "acc123".into(),
            access_key_id: "keyABC".into(),
            secret_access_key: "secretXYZ".into(),
            bucket_name: "bucket".into(),
            endpoint: String::new(),
        };
        let sql = config.create_secret_sql("r2_test");
        assert!(sql.contains("r2_test"));
        assert!(sql.contains("keyABC"));
        assert!(sql.contains("secretXYZ"));
        assert!(sql.contains("acc123.r2.cloudflarestorage.com"));
        assert!(sql.contains("TYPE s3"));
        assert!(sql.contains("URL_STYLE 'path'"));
    }

    #[test]
    fn create_secret_sql_motherduck_contains_in_motherduck() {
        let config = R2Config {
            account_id: "acc123".into(),
            access_key_id: "keyABC".into(),
            secret_access_key: "secretXYZ".into(),
            bucket_name: "bucket".into(),
            endpoint: String::new(),
        };
        let sql = config.create_secret_sql_motherduck("r2_zenith");
        assert!(sql.contains("IN MOTHERDUCK"));
        assert!(sql.contains("r2_zenith"));
    }
}
