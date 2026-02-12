//! Shared test utilities for zen-db integration tests.

#[cfg(test)]
pub(crate) mod helpers {
    use crate::ZenDb;
    use crate::service::ZenService;
    use crate::trail::writer::TrailWriter;

    /// Create an in-memory ZenService with trail disabled (for pure DB tests).
    pub async fn test_service() -> ZenService {
        let db = ZenDb::open_local(":memory:").await.unwrap();
        ZenService::from_db(db, TrailWriter::disabled())
    }

    /// Create an in-memory ZenService with trail enabled writing to a temp dir.
    pub async fn test_service_with_trail(trail_dir: std::path::PathBuf) -> ZenService {
        let db = ZenDb::open_local(":memory:").await.unwrap();
        let trail = TrailWriter::new(trail_dir).unwrap();
        ZenService::from_db(db, trail)
    }

    /// Start a session and return its ID (convenience for tests that need a session).
    pub async fn start_test_session(svc: &ZenService) -> String {
        let (session, _) = svc.start_session().await.unwrap();
        session.id
    }
}
