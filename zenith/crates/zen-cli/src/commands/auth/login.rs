use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::auth::AuthLoginArgs;
use crate::output::output;

#[derive(Serialize)]
struct AuthLoginResponse {
    authenticated: bool,
    user_id: String,
    org_id: Option<String>,
    org_slug: Option<String>,
    expires_at: String,
}

pub async fn handle(
    args: &AuthLoginArgs,
    flags: &GlobalFlags,
    config: &zen_config::ZenConfig,
) -> anyhow::Result<()> {
    let secret_key = &config.clerk.secret_key;

    if secret_key.is_empty() {
        anyhow::bail!("auth login: ZENITH_CLERK__SECRET_KEY is not configured");
    }

    let claims = if args.api_key {
        let user_id = args
            .user_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("auth login --api-key requires --user-id"))?;
        zen_auth::api_key::login_with_api_key(secret_key, user_id).await?
    } else {
        let frontend_api = resolve_frontend_api(&config)?;
        zen_auth::browser_flow::login(
            &frontend_api,
            secret_key,
            std::time::Duration::from_secs(120),
            None,
        )
        .await?
    };

    output(
        &AuthLoginResponse {
            authenticated: true,
            user_id: claims.user_id,
            org_id: claims.org_id,
            org_slug: claims.org_slug,
            expires_at: claims.expires_at.to_rfc3339(),
        },
        flags.format,
    )
}

/// Resolve the Clerk frontend API hostname.
///
/// Priority: `config.clerk.frontend_url` → extract from JWKS URL hostname.
pub(crate) fn resolve_frontend_api(config: &zen_config::ZenConfig) -> anyhow::Result<String> {
    if !config.clerk.frontend_url.is_empty() {
        return normalize_frontend_host(&config.clerk.frontend_url);
    }

    let jwks_url = &config.clerk.jwks_url;
    if jwks_url.is_empty() {
        anyhow::bail!(
            "cannot determine Clerk frontend URL — set ZENITH_CLERK__FRONTEND_URL or ZENITH_CLERK__JWKS_URL"
        );
    }

    // Extract hostname from JWKS URL
    // e.g., https://ruling-doe-21.clerk.accounts.dev/.well-known/jwks.json → ruling-doe-21.clerk.accounts.dev
    let host = jwks_url
        .strip_prefix("https://")
        .or_else(|| jwks_url.strip_prefix("http://"))
        .and_then(|rest| rest.split('/').next())
        .ok_or_else(|| {
            anyhow::anyhow!("invalid JWKS URL format — set ZENITH_CLERK__FRONTEND_URL")
        })?;

    Ok(host.to_string())
}

fn normalize_frontend_host(value: &str) -> anyhow::Result<String> {
    let host = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or(value)
        .split('/')
        .next()
        .unwrap_or("")
        .trim();

    if host.is_empty() {
        anyhow::bail!("invalid Clerk frontend URL format");
    }

    if let Some(stripped) = host.strip_suffix(".clerk.accounts.dev") {
        return Ok(format!("{stripped}.accounts.dev"));
    }

    Ok(host.to_string())
}

#[cfg(test)]
mod tests {
    use super::normalize_frontend_host;

    #[test]
    fn normalize_frontend_host_accepts_bare_hostname() {
        let host = normalize_frontend_host("ruling-doe-21.accounts.dev").expect("should parse");
        assert_eq!(host, "ruling-doe-21.accounts.dev");
    }

    #[test]
    fn normalize_frontend_host_strips_scheme_and_path() {
        let host = normalize_frontend_host("https://ruling-doe-21.accounts.dev/user")
            .expect("should parse");
        assert_eq!(host, "ruling-doe-21.accounts.dev");
    }

    #[test]
    fn normalize_frontend_host_maps_clerk_dev_domain() {
        let host = normalize_frontend_host("https://ruling-doe-21.clerk.accounts.dev")
            .expect("should parse");
        assert_eq!(host, "ruling-doe-21.accounts.dev");
    }
}
