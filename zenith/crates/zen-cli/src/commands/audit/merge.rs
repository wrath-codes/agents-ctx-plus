use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TimelineEntry {
    pub source: String,
    pub created_at: String,
    pub entry: serde_json::Value,
}

pub fn merge_timeline(
    entity_audit: &[zen_core::entities::AuditEntry],
    file_audit: &[zen_core::workspace::WorkspaceAuditEntry],
) -> anyhow::Result<Vec<TimelineEntry>> {
    let mut merged = Vec::with_capacity(entity_audit.len() + file_audit.len());

    for entry in entity_audit {
        merged.push(TimelineEntry {
            source: "entity".to_string(),
            created_at: entry.created_at.to_rfc3339(),
            entry: serde_json::to_value(entry)?,
        });
    }

    for entry in file_audit {
        merged.push(TimelineEntry {
            source: "file".to_string(),
            created_at: entry.created_at.to_rfc3339(),
            entry: serde_json::to_value(entry)?,
        });
    }

    merged.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.source.cmp(&b.source))
    });

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::entities::AuditEntry;
    use zen_core::enums::{AuditAction, EntityType};
    use zen_core::workspace::WorkspaceAuditEntry;

    use super::merge_timeline;

    #[test]
    fn sorts_merged_entries_by_timestamp_then_source() {
        let entity = AuditEntry {
            id: "aud-1".to_string(),
            session_id: Some("ses-1".to_string()),
            entity_type: EntityType::Task,
            entity_id: "tsk-1".to_string(),
            action: AuditAction::Created,
            detail: None,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:01Z")
                .expect("valid timestamp")
                .with_timezone(&Utc),
        };
        let file = WorkspaceAuditEntry {
            id: "wsa-1".to_string(),
            session_id: "ses-1".to_string(),
            workspace_id: "ws-ses-1".to_string(),
            source: "file".to_string(),
            event: "write".to_string(),
            path: Some("/workspace/src/lib.rs".to_string()),
            tool: "install_index".to_string(),
            status: "success".to_string(),
            params: None,
            result: None,
            error: None,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .expect("valid timestamp")
                .with_timezone(&Utc),
        };

        let merged = merge_timeline(&[entity], &[file]).expect("merge should succeed");
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].source, "file");
        assert_eq!(merged[1].source, "entity");
    }
}
