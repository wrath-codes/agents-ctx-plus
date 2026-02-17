use serde::Serialize;
use zen_core::entities::Session;

#[derive(Debug, Serialize)]
pub struct SessionStartResponse {
    pub session: Session,
    pub orphaned: Option<Session>,
}
