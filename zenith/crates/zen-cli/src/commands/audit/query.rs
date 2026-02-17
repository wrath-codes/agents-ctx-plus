use zen_core::enums::{AuditAction, EntityType};
use zen_db::repos::audit::AuditFilter;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::AuditArgs;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(args: &AuditArgs, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let entries = fetch(args, ctx, flags).await?;
    output(&entries, flags.format)
}

pub async fn fetch(
    args: &AuditArgs,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<Vec<zen_core::entities::AuditEntry>> {
    let limit = effective_limit(None, flags.limit, 50);
    let filter = AuditFilter {
        entity_type: args
            .entity_type
            .as_deref()
            .map(|value| parse_enum::<EntityType>(value, "entity-type"))
            .transpose()?,
        entity_id: args.entity_id.clone(),
        action: args
            .action
            .as_deref()
            .map(|value| parse_enum::<AuditAction>(value, "action"))
            .transpose()?,
        session_id: args.session.clone(),
        limit: Some(limit),
    };

    ctx.service.query_audit(&filter).await.map_err(Into::into)
}
