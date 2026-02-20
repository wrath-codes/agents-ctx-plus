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

pub async fn handle(args: &AuthLoginArgs, flags: &GlobalFlags) -> anyhow::Result<()> {
    let config = zen_config::ZenConfig::load().map_err(anyhow::Error::from)?;
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
        return Ok(config.clerk.frontend_url.clone());
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
        .ok_or_else(|| anyhow::anyhow!("invalid JWKS URL format — set ZENITH_CLERK__FRONTEND_URL"))?;

    Ok(host.to_string())
}
