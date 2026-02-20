use anyhow::bail;
use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::cli::subcommands::team::TeamInviteArgs;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct InviteResponse {
    email: String,
    role: String,
    status: String,
    invitation_id: String,
}

pub async fn handle(
    args: &TeamInviteArgs,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let identity = ctx
        .identity
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("team invite requires authentication — run `znt auth login`"))?;
    let org_id = identity
        .org_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("team invite requires an active organization — run `znt auth switch-org <slug>`"))?;

    if ctx.config.clerk.secret_key.is_empty() {
        bail!("team invite requires ZENITH_CLERK__SECRET_KEY to be configured");
    }

    let invitation = zen_auth::org::invite_member(
        &ctx.config.clerk.secret_key,
        org_id,
        &args.email,
        &args.role,
    )
    .await?;

    output(
        &InviteResponse {
            email: args.email.clone(),
            role: args.role.clone(),
            status: invitation.status,
            invitation_id: invitation.id,
        },
        flags.format,
    )
}
