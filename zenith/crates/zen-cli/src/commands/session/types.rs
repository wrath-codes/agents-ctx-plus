use serde::Serialize;
use zen_core::entities::Session;
use zen_core::workspace::WorkspaceInfo;

#[derive(Debug, Serialize)]
pub struct SessionStartResponse {
    pub session: Session,
    pub orphaned: Option<Session>,
    pub workspace: WorkspaceInfo,
}
