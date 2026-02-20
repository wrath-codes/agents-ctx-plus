use anyhow::bail;
use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct ListResponse {
    org_id: String,
    org_slug: Option<String>,
    members: Vec<zen_auth::org::OrgMember>,
    count: usize,
}

pub async fn handle(ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let identity = ctx
        .identity
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("team list requires authentication — run `znt auth login`"))?;
    let org_id = identity
        .org_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("team list requires an active organization — run `znt auth switch-org <slug>`"))?;

    if ctx.config.clerk.secret_key.is_empty() {
        bail!("team list requires ZENITH_CLERK__SECRET_KEY to be configured");
    }

    let members = zen_auth::org::list_members(&ctx.config.clerk.secret_key, org_id).await?;

    let count = members.len();
    output(
        &ListResponse {
            org_id: org_id.to_string(),
            org_slug: identity.org_slug.clone(),
            members,
            count,
        },
        flags.format,
    )
}
