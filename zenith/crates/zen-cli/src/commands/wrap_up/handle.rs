use serde::Serialize;
use zen_db::repos::audit::AuditFilter;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::WrapUpArgs;
use crate::commands::shared::session::require_active_session_id;
use crate::commands::wrap_up::summary::resolve_summary;
use crate::commands::wrap_up::sync::{WrapUpSyncStatus, build_sync_status};
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct WrapUpSession {
    id: String,
    status: String,
    summary: String,
}

#[derive(Debug, Serialize)]
struct WrapUpResponse {
    session: WrapUpSession,
    snapshot: zen_core::entities::SessionSnapshot,
    audit_count: usize,
    sync: WrapUpSyncStatus,
}

pub async fn run(
    args: &WrapUpArgs,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let summary = resolve_summary(args);

    let snapshot = ctx.service.create_snapshot(&session_id, &summary).await?;
    let session = ctx.service.end_session(&session_id, &summary).await?;
    let audit = ctx
        .service
        .query_audit(&AuditFilter {
            session_id: Some(session_id),
            limit: Some(200),
            ..Default::default()
        })
        .await?;

    output(
        &WrapUpResponse {
            session: WrapUpSession {
                id: session.id,
                status: session.status.as_str().to_string(),
                summary,
            },
            snapshot,
            audit_count: audit.len(),
            sync: build_sync_status(args.auto_commit, args.message.as_deref()),
        },
        flags.format,
    )
}
