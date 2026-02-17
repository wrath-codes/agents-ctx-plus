use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct WrapUpSyncStatus {
    pub status: &'static str,
    pub turso_synced: bool,
    pub audit_exported: bool,
    pub git_committed: bool,
    pub note: String,
}

pub fn build_sync_status(auto_commit_requested: bool, message: Option<&str>) -> WrapUpSyncStatus {
    let note = if auto_commit_requested {
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

    WrapUpSyncStatus {
        status: "local_only",
        turso_synced: false,
        audit_exported: false,
        git_committed: false,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::build_sync_status;

    #[test]
    fn reports_deferred_auto_commit_note() {
        let status = build_sync_status(true, Some("wrap-up commit"));
        assert!(!status.git_committed);
        assert!(status.note.contains("later phase"));
        assert!(status.note.contains("wrap-up commit"));
    }
}
