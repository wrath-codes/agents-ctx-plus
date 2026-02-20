use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::output::output;

#[derive(Serialize)]
struct AuthStatusResponse {
    authenticated: bool,
    user: Option<String>,
    organization: Option<String>,
    user_id: Option<String>,
    org_id: Option<String>,
    org_slug: Option<String>,
    expires_at: Option<String>,
    token_source: Option<String>,
    note: Option<String>,
}

pub async fn handle(flags: &GlobalFlags, config: &zen_config::ZenConfig) -> anyhow::Result<()> {
    let secret_key = &config.clerk.secret_key;

    let status = if secret_key.is_empty() {
        AuthStatusResponse {
            authenticated: false,
            user: None,
            organization: None,
            user_id: None,
            org_id: None,
            org_slug: None,
            expires_at: None,
            token_source: None,
            note: Some("ZENITH_CLERK__SECRET_KEY not configured".into()),
        }
    } else {
        match zen_auth::resolve_and_validate(secret_key).await {
            Ok(Some(claims)) => AuthStatusResponse {
                authenticated: true,
                user: Some(claims.user_id.clone()),
                organization: claims.org_slug.clone().or_else(|| claims.org_id.clone()),
                user_id: Some(claims.user_id),
                org_id: claims.org_id,
                org_slug: claims.org_slug,
                expires_at: Some(claims.expires_at.to_rfc3339()),
                token_source: zen_auth::token_store::detect_token_source(),
                note: None,
            },
            Ok(None) => AuthStatusResponse {
                authenticated: false,
                user: None,
                organization: None,
                user_id: None,
                org_id: None,
                org_slug: None,
                expires_at: None,
                token_source: None,
                note: Some("no valid token found".into()),
            },
            Err(error) => AuthStatusResponse {
                authenticated: false,
                user: None,
                organization: None,
                user_id: None,
                org_id: None,
                org_slug: None,
                expires_at: None,
                token_source: None,
                note: Some(error.to_string()),
            },
        }
    };

    output(&status, flags.format)
}
