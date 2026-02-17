use zen_core::enums::SessionStatus;

use crate::context::AppContext;

/// Resolve the current active session ID.
pub async fn require_active_session_id(ctx: &AppContext) -> anyhow::Result<String> {
    let sessions = ctx
        .service
        .list_sessions(Some(SessionStatus::Active), 1)
        .await?;

    sessions
        .first()
        .map(|session| session.id.clone())
        .ok_or_else(|| anyhow::anyhow!("No active session. Run 'znt session start' first."))
}
