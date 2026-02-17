use zen_db::repos::audit::AuditFilter;

use crate::cli::GlobalFlags;
use crate::cli::OutputFormat;
use crate::context::AppContext;
use crate::output::output;

/// Handle `znt whats-next`.
pub async fn handle(ctx: &mut AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    match flags.format {
        OutputFormat::Raw => {
            let entries = ctx
                .service
                .query_audit(&AuditFilter {
                    limit: Some(flags.limit.unwrap_or(20)),
                    ..Default::default()
                })
                .await?;

            print!("{}", to_ndjson(&entries)?);

            Ok(())
        }
        OutputFormat::Json | OutputFormat::Table => {
            let state = ctx.service.whats_next().await?;
            output(&state, flags.format)
        }
    }
}

fn to_ndjson(entries: &[zen_core::entities::AuditEntry]) -> anyhow::Result<String> {
    let mut out = String::new();
    for entry in entries {
        out.push_str(&serde_json::to_string(entry)?);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::entities::AuditEntry;
    use zen_core::enums::{AuditAction, EntityType};

    use super::to_ndjson;

    #[test]
    fn ndjson_emits_one_line_per_entry() {
        let entries = vec![AuditEntry {
            id: "aud-1".to_string(),
            session_id: Some("ses-1".to_string()),
            entity_type: EntityType::Task,
            entity_id: "tsk-1".to_string(),
            action: AuditAction::Created,
            detail: None,
            created_at: Utc::now(),
        }];

        let ndjson = to_ndjson(&entries).expect("ndjson conversion should work");
        assert_eq!(ndjson.lines().count(), 1);
        assert!(ndjson.contains("\"id\":\"aud-1\""));
    }
}
