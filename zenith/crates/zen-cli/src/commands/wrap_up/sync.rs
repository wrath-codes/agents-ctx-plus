use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Synced,
    Degraded,
    LocalOnly,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct WrapUpSyncStatus {
    pub require_sync: bool,
    pub status: SyncStatus,
    pub turso_synced: bool,
    pub catalog_updated: bool,
    pub audit_exported: bool,
    pub git_committed: bool,
    pub error: Option<String>,
    pub note: String,
}

pub fn build_sync_status(
    require_sync: bool,
    auto_commit_requested: bool,
    message: Option<&str>,
    sync_result: Option<anyhow::Result<()>>,
) -> WrapUpSyncStatus {
    let auto_commit_note = if auto_commit_requested {
        let msg = message
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("(auto)");
        format!(
            "auto-commit requested (message: {msg}), but commit wiring is planned for a later phase"
        )
    } else {
        "cloud sync and commit automation are planned for later phases".to_string()
    };

    let (status, turso_synced, catalog_updated, audit_exported, error, note_prefix) =
        match sync_result {
            Some(Ok(())) => (
                SyncStatus::Synced,
                true,
                false,
                true,
                None,
                "Cloud sync completed.".to_string(),
            ),
            Some(Err(error)) => {
                let err = error.to_string();
                if require_sync {
                    (
                        SyncStatus::Failed,
                        false,
                        false,
                        false,
                        Some(err),
                        "Cloud sync failed and strict sync is required.".to_string(),
                    )
                } else {
                    (
                        SyncStatus::Degraded,
                        false,
                        false,
                        false,
                        Some(err),
                        "Local wrap-up completed; cloud sync failed.".to_string(),
                    )
                }
            }
            None => (
                SyncStatus::LocalOnly,
                false,
                false,
                false,
                None,
                "Cloud sync not attempted.".to_string(),
            ),
        };

    let note = format!("{note_prefix} {auto_commit_note}");

    WrapUpSyncStatus {
        require_sync,
        status,
        turso_synced,
        catalog_updated,
        audit_exported,
        git_committed: false,
        error,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_sync_status, SyncStatus};

    #[test]
    fn reports_deferred_auto_commit_note() {
        let status = build_sync_status(false, true, Some("wrap-up commit"), None);
        assert!(!status.git_committed);
        assert!(matches!(status.status, SyncStatus::LocalOnly));
        assert!(status.note.contains("later phase"));
        assert!(status.note.contains("wrap-up commit"));
    }

    #[test]
    fn reports_synced_when_sync_succeeds() {
        let status = build_sync_status(false, false, None, Some(Ok(())));
        assert!(matches!(status.status, SyncStatus::Synced));
        assert!(status.turso_synced);
        assert!(!status.catalog_updated);
        assert!(status.audit_exported);
        assert!(status.error.is_none());
    }

    #[test]
    fn reports_degraded_when_optional_sync_fails() {
        let status = build_sync_status(
            false,
            false,
            None,
            Some(Err(anyhow::anyhow!("sync failed"))),
        );
        assert!(matches!(status.status, SyncStatus::Degraded));
        assert!(!status.turso_synced);
        assert_eq!(status.error.as_deref(), Some("sync failed"));
    }
}
