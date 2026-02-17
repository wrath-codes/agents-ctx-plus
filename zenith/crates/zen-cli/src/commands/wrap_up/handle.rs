use serde::Serialize;
use zen_core::workspace::{WorkspaceChannelStatus, WorkspaceSnapshot};
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
    workspace_snapshot_status: WorkspaceChannelStatus,
    workspace_snapshot: Option<WorkspaceSnapshot>,
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

    let (workspace_snapshot_status, workspace_snapshot) =
        match crate::workspace::agentfs::session_workspace_snapshot(&ctx.project_root, &session.id)
            .await
        {
            Ok(snapshot) => (
                WorkspaceChannelStatus {
                    status: "ok".to_string(),
                    error: None,
                },
                Some(snapshot),
            ),
            Err(error) => (
                WorkspaceChannelStatus {
                    status: "error".to_string(),
                    error: Some(error.to_string()),
                },
                None,
            ),
        };

    output(
        &WrapUpResponse {
            session: WrapUpSession {
                id: session.id,
                status: session.status.as_str().to_string(),
                summary,
            },
            snapshot,
            audit_count: audit.len(),
            workspace_snapshot_status,
            workspace_snapshot,
            sync: build_sync_status(args.auto_commit, args.message.as_deref()),
        },
        flags.format,
    )
}
