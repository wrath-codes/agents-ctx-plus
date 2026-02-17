use crate::cli::GlobalFlags;
use crate::context::AppContext;
use crate::output::output;

use super::types::SessionStartResponse;

pub async fn run(ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let (session, orphaned) = ctx.service.start_session().await?;
    let workspace =
        match crate::workspace::agentfs::create_session_workspace(&ctx.project_root, &session.id)
            .await
        {
            Ok(workspace) => workspace,
            Err(error) => {
                if let Err(abandon_error) = ctx.service.abandon_session(&session.id).await {
                    tracing::error!(
                        session = %session.id,
                        %abandon_error,
                        "session start: failed to abandon session after workspace init failure"
                    );
                }
                return Err(error);
            }
        };
    output(
        &SessionStartResponse {
            session,
            orphaned,
            workspace,
        },
        flags.format,
    )
}
