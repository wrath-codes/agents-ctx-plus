use serde::Serialize;
use zen_core::workspace::{WorkspaceAuditEntry, WorkspaceChannelStatus};

use crate::cli::GlobalFlags;
use crate::cli::root_commands::AuditArgs;
use crate::commands::audit::merge::merge_timeline;
use crate::commands::shared::limit::effective_limit;
use crate::context::AppContext;
use crate::output::output;

use super::{query, search};

#[derive(Debug, Serialize)]
struct AuditFilesMeta {
    session: Option<String>,
    limit: u32,
    files: bool,
    merge_timeline: bool,
}

#[derive(Debug, Serialize)]
struct AuditFilesResponse {
    entity_audit_status: WorkspaceChannelStatus,
    file_audit_status: WorkspaceChannelStatus,
    entity_audit: Vec<zen_core::entities::AuditEntry>,
    file_audit: Vec<WorkspaceAuditEntry>,
    meta: AuditFilesMeta,
}

#[derive(Debug, Serialize)]
struct AuditFilesMergedResponse {
    entity_audit_status: WorkspaceChannelStatus,
    file_audit_status: WorkspaceChannelStatus,
    timeline: Vec<crate::commands::audit::merge::TimelineEntry>,
    meta: AuditFilesMeta,
}

pub async fn run(args: &AuditArgs, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let limit = effective_limit(None, flags.limit, 50);

    let (entity_status, entity_entries) = match args.search.as_deref() {
        Some(query_text) => match search::fetch(query_text, ctx, flags).await {
            Ok(entries) => (ok_status(), entries),
            Err(error) => (error_status(error), Vec::new()),
        },
        None => match query::fetch(args, ctx, flags).await {
            Ok(entries) => (ok_status(), entries),
            Err(error) => (error_status(error), Vec::new()),
        },
    };

    let requested_session = args.session.clone();
    let (file_status, file_entries) = if let Some(session_id) = requested_session.as_deref() {
        match crate::workspace::agentfs::session_file_audit(
            &ctx.project_root,
            session_id,
            limit,
            args.search.as_deref(),
        )
        .await
        {
            Ok(entries) => (ok_status(), entries),
            Err(error) => (error_status(error), Vec::new()),
        }
    } else {
        match crate::workspace::agentfs::active_session_file_audit(
            &ctx.project_root,
            limit,
            args.search.as_deref(),
        )
        .await
        {
            Ok(entries) => (ok_status(), entries),
            Err(error) => (error_status(error), Vec::new()),
        }
    };

    if entity_status.status == "error" && file_status.status == "error" {
        anyhow::bail!(
            "audit --files: both channels failed (entity={}, file={})",
            entity_status.error.as_deref().unwrap_or("unknown"),
            file_status.error.as_deref().unwrap_or("unknown")
        );
    }

    let meta = AuditFilesMeta {
        session: requested_session,
        limit,
        files: true,
        merge_timeline: args.merge_timeline,
    };

    if args.merge_timeline {
        let timeline = merge_timeline(&entity_entries, &file_entries)?;
        return output(
            &AuditFilesMergedResponse {
                entity_audit_status: entity_status,
                file_audit_status: file_status,
                timeline,
                meta,
            },
            flags.format,
        );
    }

    output(
        &AuditFilesResponse {
            entity_audit_status: entity_status,
            file_audit_status: file_status,
            entity_audit: entity_entries,
            file_audit: file_entries,
            meta,
        },
        flags.format,
    )
}

fn ok_status() -> WorkspaceChannelStatus {
    WorkspaceChannelStatus {
        status: "ok".to_string(),
        error: None,
    }
}

fn error_status(error: anyhow::Error) -> WorkspaceChannelStatus {
    WorkspaceChannelStatus {
        status: "error".to_string(),
        error: Some(error.to_string()),
    }
}
