use std::path::Path;

use zen_core::enums::{EntityType, TrailOp};
use zen_core::responses::RebuildResponse;
use zen_core::trail::TrailOperation;
use zen_schema::SchemaRegistry;

use crate::ZenDb;
use crate::error::DatabaseError;
use crate::helpers::entity_type_to_table;
use crate::service::ZenService;

pub struct TrailReplayer;

impl TrailReplayer {
    pub async fn rebuild(
        service: &mut ZenService,
        trail_dir: &Path,
        strict: bool,
    ) -> Result<RebuildResponse, DatabaseError> {
        let start = std::time::Instant::now();

        service.trail_mut().set_enabled(false);

        let mut trail_files = 0u32;
        let mut all_ops: Vec<TrailOperation> = Vec::new();

        let entries = std::fs::read_dir(trail_dir).map_err(|e| DatabaseError::Other(e.into()))?;

        for entry in entries {
            let entry = entry.map_err(|e| DatabaseError::Other(e.into()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            trail_files += 1;

            let ops: Vec<TrailOperation> = serde_jsonlines::json_lines(&path)
                .map_err(|e| DatabaseError::Other(e.into()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| DatabaseError::Other(e.into()))?;
            all_ops.extend(ops);
        }

        all_ops.sort_by(|a, b| a.ts.cmp(&b.ts));

        let schema = if strict {
            Some(SchemaRegistry::new())
        } else {
            None
        };

        let mut operations_replayed = 0u32;
        let mut entities_created = 0u32;

        for op in &all_ops {
            if op.v != 1 {
                return Err(DatabaseError::InvalidState(format!(
                    "Unsupported trail version {} for op {}",
                    op.v, op.id
                )));
            }

            if strict {
                if let Some(ref s) = schema {
                    if op.op == TrailOp::Create {
                        let schema_name = entity_type_to_schema_name(op.entity);
                        if let Err(e) = s.validate(schema_name, &op.data) {
                            tracing::warn!(
                                "Schema validation failed for {} {}: {:?}",
                                op.entity,
                                op.id,
                                e
                            );
                        }
                    }
                }
            }

            replay_operation(service.db(), op).await?;
            operations_replayed += 1;

            if op.op == TrailOp::Create {
                entities_created += 1;
            }
        }

        service.trail_mut().set_enabled(true);

        Ok(RebuildResponse {
            rebuilt: true,
            trail_files,
            operations_replayed,
            entities_created,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn json_to_value(data: &serde_json::Value, field: &str) -> libsql::Value {
    match data.get(field) {
        None | Some(serde_json::Value::Null) => libsql::Value::Null,
        Some(serde_json::Value::String(s)) => libsql::Value::Text(s.clone()),
        Some(v) => libsql::Value::Text(v.to_string()),
    }
}

fn json_to_update_value(data: &serde_json::Value, field: &str) -> Option<libsql::Value> {
    match data.get(field) {
        None => None,
        Some(serde_json::Value::Null) => Some(libsql::Value::Null),
        Some(serde_json::Value::String(s)) => Some(libsql::Value::Text(s.clone())),
        Some(v) => Some(libsql::Value::Text(v.to_string())),
    }
}

fn json_int_or_null(data: &serde_json::Value, field: &str) -> libsql::Value {
    match data.get(field) {
        None | Some(serde_json::Value::Null) => libsql::Value::Null,
        Some(v) => v
            .as_i64()
            .map(libsql::Value::Integer)
            .unwrap_or(libsql::Value::Null),
    }
}

async fn replay_operation(db: &ZenDb, op: &TrailOperation) -> Result<(), DatabaseError> {
    match (&op.op, &op.entity) {
        (TrailOp::Create, EntityType::Session) => {
            db.conn()
                .execute(
                    "INSERT OR IGNORE INTO sessions (id, started_at, status, summary) VALUES (?1, ?2, ?3, ?4)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        libsql::Value::Text(
                            op.data.get("started_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string(),
                        ),
                        libsql::Value::Text(
                            op.data.get("status").and_then(|v| v.as_str()).unwrap_or("active").to_string(),
                        ),
                        json_to_value(&op.data, "summary"),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::Research) => {
            db.execute(
                    "INSERT OR IGNORE INTO research_items (id, session_id, title, description, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        libsql::Value::Text(op.data["title"].as_str().unwrap_or("").to_string()),
                        json_to_value(&op.data, "description"),
                        libsql::Value::Text(op.data.get("status").and_then(|v| v.as_str()).unwrap_or("open").to_string()),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                        libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::Finding) => {
            db.execute(
                    "INSERT OR IGNORE INTO findings (id, research_id, session_id, content, source, confidence, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        json_to_value(&op.data, "research_id"),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        libsql::Value::Text(op.data["content"].as_str().unwrap_or("").to_string()),
                        json_to_value(&op.data, "source"),
                        libsql::Value::Text(op.data.get("confidence").and_then(|v| v.as_str()).unwrap_or("medium").to_string()),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                        libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::Hypothesis) => {
            db.execute(
                    "INSERT OR IGNORE INTO hypotheses (id, research_id, finding_id, session_id, content, status, reason, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        json_to_value(&op.data, "research_id"),
                        json_to_value(&op.data, "finding_id"),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        libsql::Value::Text(op.data["content"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data.get("status").and_then(|v| v.as_str()).unwrap_or("unverified").to_string()),
                        json_to_value(&op.data, "reason"),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                        libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::Insight) => {
            db.execute(
                    "INSERT OR IGNORE INTO insights (id, research_id, session_id, content, confidence, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        json_to_value(&op.data, "research_id"),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        libsql::Value::Text(op.data["content"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data.get("confidence").and_then(|v| v.as_str()).unwrap_or("medium").to_string()),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                        libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::Issue) => {
            let issue_type = op
                .data
                .get("issue_type")
                .or_else(|| op.data.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("bug");
            let priority = op
                .data
                .get("priority")
                .and_then(|v| v.as_i64())
                .unwrap_or(3);

            db.execute(
                    "INSERT OR IGNORE INTO issues (id, type, parent_id, title, description, status, priority, session_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        libsql::Value::Text(issue_type.to_string()),
                        json_to_value(&op.data, "parent_id"),
                        libsql::Value::Text(op.data["title"].as_str().unwrap_or("").to_string()),
                        json_to_value(&op.data, "description"),
                        libsql::Value::Text(op.data.get("status").and_then(|v| v.as_str()).unwrap_or("open").to_string()),
                        libsql::Value::Integer(priority),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                        libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::Task) => {
            db.execute(
                    "INSERT OR IGNORE INTO tasks (id, research_id, issue_id, session_id, title, description, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        json_to_value(&op.data, "research_id"),
                        json_to_value(&op.data, "issue_id"),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        libsql::Value::Text(op.data["title"].as_str().unwrap_or("").to_string()),
                        json_to_value(&op.data, "description"),
                        libsql::Value::Text(op.data.get("status").and_then(|v| v.as_str()).unwrap_or("open").to_string()),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                        libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::ImplLog) => {
            db.execute(
                    "INSERT OR IGNORE INTO implementation_log (id, task_id, session_id, file_path, start_line, end_line, description, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        libsql::Value::Text(op.data["task_id"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        libsql::Value::Text(op.data["file_path"].as_str().unwrap_or("").to_string()),
                        json_int_or_null(&op.data, "start_line"),
                        json_int_or_null(&op.data, "end_line"),
                        json_to_value(&op.data, "description"),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::Compat) => {
            db.execute(
                    "INSERT OR IGNORE INTO compatibility_checks (id, package_a, package_b, status, conditions, finding_id, session_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        libsql::Value::Text(op.data["package_a"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data["package_b"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string()),
                        json_to_value(&op.data, "conditions"),
                        json_to_value(&op.data, "finding_id"),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                        libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Create, EntityType::Study) => {
            db.execute(
                    "INSERT OR IGNORE INTO studies (id, session_id, research_id, topic, library, methodology, status, summary, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        libsql::Value::Text(op.data["session_id"].as_str().unwrap_or(&op.ses).to_string()),
                        json_to_value(&op.data, "research_id"),
                        libsql::Value::Text(op.data["topic"].as_str().unwrap_or("").to_string()),
                        json_to_value(&op.data, "library"),
                        libsql::Value::Text(op.data.get("methodology").and_then(|v| v.as_str()).unwrap_or("explore").to_string()),
                        libsql::Value::Text(op.data.get("status").and_then(|v| v.as_str()).unwrap_or("active").to_string()),
                        json_to_value(&op.data, "summary"),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                        libsql::Value::Text(op.data.get("updated_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Update, entity) => {
            replay_update(db, entity, &op.id, &op.data, &op.ts).await?;
        }

        (TrailOp::Transition, entity) => {
            let table = entity_type_to_table(entity);
            let new_status = op.data["to"].as_str().ok_or_else(|| {
                DatabaseError::InvalidState("Transition missing 'to' field".into())
            })?;

            match entity {
                EntityType::Session => {
                    let ended_at = op
                        .data
                        .get("ended_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&op.ts);
                    let summary = json_to_value(&op.data, "summary");
                    db.execute(
                            &format!("UPDATE {table} SET status = ?1, ended_at = ?2, summary = ?3 WHERE id = ?4"),
                            vec![
                                libsql::Value::Text(new_status.to_string()),
                                libsql::Value::Text(ended_at.to_string()),
                                summary,
                                libsql::Value::Text(op.id.clone()),
                            ],
                        )
                        .await?;
                }
                EntityType::Hypothesis => {
                    let reason = json_to_value(&op.data, "reason");
                    db.execute(
                            &format!("UPDATE {table} SET status = ?1, reason = ?2, updated_at = ?3 WHERE id = ?4"),
                            vec![
                                libsql::Value::Text(new_status.to_string()),
                                reason,
                                libsql::Value::Text(op.ts.clone()),
                                libsql::Value::Text(op.id.clone()),
                            ],
                        )
                        .await?;
                }
                _ => {
                    db.execute_with(
                        &format!("UPDATE {table} SET status = ?1, updated_at = ?2 WHERE id = ?3"),
                        || libsql::params![new_status, op.ts.as_str(), op.id.as_str()],
                    )
                    .await?;
                }
            }
        }

        (TrailOp::Delete, entity) => {
            let table = entity_type_to_table(entity);
            db.execute(
                &format!("DELETE FROM {table} WHERE id = ?1"),
                [op.id.as_str()],
            )
            .await?;
        }

        (TrailOp::Tag, EntityType::Finding) => {
            let tag = op.data["tag"].as_str().unwrap_or("");
            db.execute_with(
                "INSERT OR IGNORE INTO finding_tags (finding_id, tag) VALUES (?1, ?2)",
                || libsql::params![op.id.as_str(), tag],
            )
            .await?;
        }

        (TrailOp::Untag, EntityType::Finding) => {
            let tag = op.data["tag"].as_str().unwrap_or("");
            db.execute_with(
                "DELETE FROM finding_tags WHERE finding_id = ?1 AND tag = ?2",
                || libsql::params![op.id.as_str(), tag],
            )
            .await?;
        }

        (TrailOp::Link, EntityType::EntityLink) => {
            db.execute(
                    "INSERT OR IGNORE INTO entity_links (id, source_type, source_id, target_type, target_id, relation, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    vec![
                        libsql::Value::Text(op.id.clone()),
                        libsql::Value::Text(op.data["source_type"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data["source_id"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data["target_type"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data["target_id"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data["relation"].as_str().unwrap_or("").to_string()),
                        libsql::Value::Text(op.data.get("created_at").and_then(|v| v.as_str()).unwrap_or(&op.ts).to_string()),
                    ],
                )
                .await?;
        }

        (TrailOp::Unlink, EntityType::EntityLink) => {
            db.execute("DELETE FROM entity_links WHERE id = ?1", [op.id.as_str()])
                .await?;
        }

        (trail_op, entity) => {
            tracing::warn!(
                "Unhandled trail replay: op={}, entity={}, id={}",
                trail_op,
                entity,
                op.id
            );
        }
    }

    Ok(())
}

async fn replay_update(
    db: &ZenDb,
    entity: &EntityType,
    id: &str,
    data: &serde_json::Value,
    ts: &str,
) -> Result<(), DatabaseError> {
    let table = entity_type_to_table(entity);

    let obj = match data.as_object() {
        Some(o) => o,
        None => return Ok(()),
    };

    let has_updated_at = !matches!(
        entity,
        EntityType::Session | EntityType::ImplLog | EntityType::EntityLink
    );

    let mut sets = Vec::new();
    let mut params: Vec<libsql::Value> = Vec::new();
    let mut idx = 1u32;

    for (key, _) in obj {
        let col_name = if entity == &EntityType::Issue && key == "issue_type" {
            "type"
        } else {
            key.as_str()
        };

        if let Some(val) = json_to_update_value(data, key) {
            sets.push(format!("{col_name} = ?{idx}"));
            params.push(val);
            idx += 1;
        }
    }

    if has_updated_at {
        sets.push(format!("updated_at = ?{idx}"));
        params.push(libsql::Value::Text(ts.to_string()));
        idx += 1;
    }

    if sets.is_empty() {
        return Ok(());
    }

    let set_clause = sets.join(", ");
    params.push(libsql::Value::Text(id.to_string()));
    let sql = format!("UPDATE {table} SET {set_clause} WHERE id = ?{idx}");

    db.execute(&sql, params).await?;

    Ok(())
}

fn entity_type_to_schema_name(entity: EntityType) -> &'static str {
    match entity {
        EntityType::Session => "session",
        EntityType::Research => "research_item",
        EntityType::Finding => "finding",
        EntityType::Hypothesis => "hypothesis",
        EntityType::Insight => "insight",
        EntityType::Issue => "issue",
        EntityType::Task => "task",
        EntityType::ImplLog => "impl_log",
        EntityType::Compat => "compat_check",
        EntityType::Study => "study",
        EntityType::Decision => "decision",
        EntityType::EntityLink => "entity_link",
        EntityType::Audit => "audit_entry",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::helpers::{start_test_session, test_service_with_trail};
    use zen_core::enums::{Confidence, HypothesisStatus, Relation, TaskStatus};

    #[tokio::test]
    async fn rebuild_roundtrip() {
        let trail_dir = tempfile::tempdir().unwrap();
        let svc = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(
                &sid,
                "tokio runtime analysis",
                Some("docs"),
                Confidence::High,
                None,
            )
            .await
            .unwrap();
        let task = svc
            .create_task(&sid, "Implement feature", None, None, None)
            .await
            .unwrap();

        let finding_id = finding.id.clone();
        let task_id = task.id.clone();

        let mut svc2 = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let result = TrailReplayer::rebuild(&mut svc2, trail_dir.path(), false)
            .await
            .unwrap();

        assert!(result.rebuilt);
        assert!(result.operations_replayed > 0);
        assert!(result.entities_created > 0);

        let rebuilt_finding = svc2.get_finding(&finding_id).await.unwrap();
        assert_eq!(rebuilt_finding.content, "tokio runtime analysis");
        assert_eq!(rebuilt_finding.confidence, Confidence::High);

        let rebuilt_task = svc2.get_task(&task_id).await.unwrap();
        assert_eq!(rebuilt_task.title, "Implement feature");
        assert_eq!(rebuilt_task.status, TaskStatus::Open);
    }

    #[tokio::test]
    async fn rebuild_multi_session() {
        let trail_dir = tempfile::tempdir().unwrap();

        let svc1 = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let sid1 = start_test_session(&svc1).await;
        let f1 = svc1
            .create_finding(
                &sid1,
                "finding from session 1",
                None,
                Confidence::Medium,
                None,
            )
            .await
            .unwrap();

        let svc_other = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let sid2 = start_test_session(&svc_other).await;
        let f2 = svc_other
            .create_finding(&sid2, "finding from session 2", None, Confidence::Low, None)
            .await
            .unwrap();

        let mut svc_rebuild = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let result = TrailReplayer::rebuild(&mut svc_rebuild, trail_dir.path(), false)
            .await
            .unwrap();

        assert_eq!(result.trail_files, 2);

        let r1 = svc_rebuild.get_finding(&f1.id).await.unwrap();
        assert_eq!(r1.content, "finding from session 1");

        let r2 = svc_rebuild.get_finding(&f2.id).await.unwrap();
        assert_eq!(r2.content, "finding from session 2");
    }

    #[tokio::test]
    async fn rebuild_fts_survives() {
        let trail_dir = tempfile::tempdir().unwrap();
        let svc = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let sid = start_test_session(&svc).await;

        svc.create_finding(
            &sid,
            "tokio async runtime compatibility",
            None,
            Confidence::High,
            None,
        )
        .await
        .unwrap();

        let mut svc2 = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        TrailReplayer::rebuild(&mut svc2, trail_dir.path(), false)
            .await
            .unwrap();

        let results = svc2.search_findings("runtime", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("runtime"));
    }

    #[tokio::test]
    async fn rebuild_version_dispatch() {
        let trail_dir = tempfile::tempdir().unwrap();
        let svc = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let sid = start_test_session(&svc).await;
        svc.create_finding(&sid, "v1 test", None, Confidence::Medium, None)
            .await
            .unwrap();

        let mut svc2 = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let result = TrailReplayer::rebuild(&mut svc2, trail_dir.path(), false)
            .await
            .unwrap();
        assert!(result.rebuilt);
    }

    #[tokio::test]
    async fn rebuild_unsupported_version() {
        let trail_dir = tempfile::tempdir().unwrap();
        let path = trail_dir.path().join("bad.jsonl");

        let op = serde_json::json!({
            "v": 99,
            "ts": "2026-02-09T10:00:00Z",
            "ses": "ses-bad",
            "op": "create",
            "entity": "finding",
            "id": "fnd-bad",
            "data": {"content": "test"}
        });
        std::fs::write(&path, format!("{}\n", op)).unwrap();

        let mut svc = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let result = TrailReplayer::rebuild(&mut svc, trail_dir.path(), false).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unsupported trail version"));
    }

    #[tokio::test]
    async fn rebuild_tag_untag_survives() {
        let trail_dir = tempfile::tempdir().unwrap();
        let svc = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let sid = start_test_session(&svc).await;

        let finding = svc
            .create_finding(&sid, "tagged finding", None, Confidence::High, None)
            .await
            .unwrap();
        svc.tag_finding(&sid, &finding.id, "important")
            .await
            .unwrap();
        svc.tag_finding(&sid, &finding.id, "verified")
            .await
            .unwrap();
        svc.untag_finding(&sid, &finding.id, "verified")
            .await
            .unwrap();

        let mut svc2 = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        TrailReplayer::rebuild(&mut svc2, trail_dir.path(), false)
            .await
            .unwrap();

        let tags = svc2.get_finding_tags(&finding.id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert!(tags.contains(&"important".to_string()));
    }

    #[tokio::test]
    async fn rebuild_link_survives() {
        let trail_dir = tempfile::tempdir().unwrap();
        let svc = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let sid = start_test_session(&svc).await;

        let link = svc
            .create_link(
                &sid,
                EntityType::Finding,
                "fnd-001",
                EntityType::Hypothesis,
                "hyp-001",
                Relation::Validates,
            )
            .await
            .unwrap();

        let mut svc2 = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        TrailReplayer::rebuild(&mut svc2, trail_dir.path(), false)
            .await
            .unwrap();

        let rebuilt_link = svc2.get_link(&link.id).await.unwrap();
        assert_eq!(rebuilt_link.source_id, "fnd-001");
        assert_eq!(rebuilt_link.target_id, "hyp-001");
        assert_eq!(rebuilt_link.relation, Relation::Validates);
    }

    #[tokio::test]
    async fn rebuild_transition_survives() {
        let trail_dir = tempfile::tempdir().unwrap();
        let svc = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        let sid = start_test_session(&svc).await;

        let hyp = svc
            .create_hypothesis(&sid, "test hypothesis", None, None)
            .await
            .unwrap();
        svc.transition_hypothesis(&sid, &hyp.id, HypothesisStatus::Analyzing, None)
            .await
            .unwrap();
        svc.transition_hypothesis(
            &sid,
            &hyp.id,
            HypothesisStatus::Confirmed,
            Some("evidence found"),
        )
        .await
        .unwrap();

        let mut svc2 = test_service_with_trail(trail_dir.path().to_path_buf()).await;
        TrailReplayer::rebuild(&mut svc2, trail_dir.path(), false)
            .await
            .unwrap();

        let rebuilt = svc2.get_hypothesis(&hyp.id).await.unwrap();
        assert_eq!(rebuilt.status, HypothesisStatus::Confirmed);
        assert_eq!(rebuilt.reason.as_deref(), Some("evidence found"));
    }

    #[tokio::test]
    async fn rebuild_empty_trail_dir() {
        let trail_dir = tempfile::tempdir().unwrap();
        let mut svc = test_service_with_trail(trail_dir.path().to_path_buf()).await;

        let result = TrailReplayer::rebuild(&mut svc, trail_dir.path(), false)
            .await
            .unwrap();

        assert!(result.rebuilt);
        assert_eq!(result.trail_files, 0);
        assert_eq!(result.operations_replayed, 0);
        assert_eq!(result.entities_created, 0);
    }
}
