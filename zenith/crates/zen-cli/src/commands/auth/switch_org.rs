use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::auth::AuthSwitchOrgArgs;
use crate::output::output;

use crate::commands::auth::login::resolve_frontend_api;

#[derive(Serialize)]
struct AuthSwitchOrgResponse {
    switched: bool,
    org_id: Option<String>,
    org_slug: Option<String>,
    org_role: Option<String>,
}

pub async fn handle(
    args: &AuthSwitchOrgArgs,
    flags: &GlobalFlags,
    config: &zen_config::ZenConfig,
) -> anyhow::Result<()> {
    let secret_key = &config.clerk.secret_key;

    if secret_key.is_empty() {
        anyhow::bail!("auth switch-org: ZENITH_CLERK__SECRET_KEY is not configured");
    }

    let frontend_api = resolve_frontend_api(&config)?;

    // Clear existing credentials before re-auth
    zen_auth::logout().ok();

    let claims = zen_auth::browser_flow::login(
        &frontend_api,
        secret_key,
        std::time::Duration::from_secs(120),
        Some(&args.org_slug),
    )
    .await?;

    if claims.org_slug.as_deref() != Some(&args.org_slug) {
        tracing::warn!(
            expected = %args.org_slug,
            actual = ?claims.org_slug,
            "org slug in JWT does not match requested org"
        );
    }

    output(
        &AuthSwitchOrgResponse {
            switched: true,
            org_id: claims.org_id,
            org_slug: claims.org_slug,
            org_role: claims.org_role,
        },
        flags.format,
    )
}
