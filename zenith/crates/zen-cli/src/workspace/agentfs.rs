use std::path::{Path, PathBuf};

use agentfs_sdk::{AgentFS, AgentFSOptions};
use anyhow::Context;
use chrono::{DateTime, Utc};
use zen_core::workspace::{
    WorkspaceAuditEntry, WorkspaceBackend, WorkspaceInfo, WorkspaceSnapshot,
};

const WORKSPACE_ROOT: &str = "/workspace";

pub async fn create_session_workspace(
    project_root: &Path,
    session_id: &str,
) -> anyhow::Result<WorkspaceInfo> {
    let db_path = persistent_workspace_db_path(project_root, session_id).await?;
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("invalid workspace path"))?;

    let agent = AgentFS::open(AgentFSOptions::with_path(db_path_str)).await?;
    let sid = session_id.to_string();
    let root = WORKSPACE_ROOT.to_string();
    agent.kv.set("session_id", &sid).await?;
    agent.kv.set("workspace_root", &root).await?;
    agent.fs.mkdir(WORKSPACE_ROOT, 0, 0).await?;

    Ok(WorkspaceInfo {
        backend: WorkspaceBackend::Agentfs,
        workspace_id: workspace_id(session_id),
        root: WORKSPACE_ROOT.to_string(),
        persistent: true,
        created: true,
        status: "ok".to_string(),
        note: None,
    })
}

pub async fn record_install_event(
    project_root: &Path,
    session_id: &str,
    ecosystem: &str,
    package: &str,
    version: &str,
    success: bool,
    error: Option<&str>,
) -> anyhow::Result<()> {
    let agent = open_session_workspace(project_root, session_id).await?;
    let now = now_epoch_secs();
    let result = if success {
        Some(serde_json::json!({
            "status": "indexed",
            "ecosystem": ecosystem,
            "package": package,
            "version": version
        }))
    } else {
        None
    };

    agent
        .tools
        .record(
            "install_index",
            now,
            now,
            Some(serde_json::json!({
                "ecosystem": ecosystem,
                "package": package,
                "version": version
            })),
            result,
            error,
        )
        .await
        .context("record workspace install event")?;
    Ok(())
}

pub async fn session_workspace_snapshot(
    project_root: &Path,
    session_id: &str,
) -> anyhow::Result<WorkspaceSnapshot> {
    let agent = open_session_workspace(project_root, session_id).await?;
    let recent = agent.tools.recent(Some(500)).await?;
    let total = u64::try_from(recent.len()).unwrap_or(u64::MAX);

    let mut success = 0u64;
    let mut failed = 0u64;
    for call in recent {
        let value = serde_json::to_value(&call)?;
        match value
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
        {
            "success" => success += 1,
            "error" => failed += 1,
            _ => {}
        }
    }

    Ok(WorkspaceSnapshot {
        status: "ok".to_string(),
        workspace_id: workspace_id(session_id),
        files_total: 0,
        bytes_total: 0,
        tool_calls_total: total,
        tool_calls_success: success,
        tool_calls_failed: failed,
        captured_at: Utc::now(),
        note: Some("file stats are pending deeper AgentFS traversal support".to_string()),
    })
}

pub async fn session_file_audit(
    project_root: &Path,
    session_id: &str,
    limit: u32,
    search: Option<&str>,
) -> anyhow::Result<Vec<WorkspaceAuditEntry>> {
    let agent = open_session_workspace(project_root, session_id).await?;
    file_audit_from_agent(&agent, session_id, limit, search).await
}

async fn file_audit_from_agent(
    agent: &AgentFS,
    session_id: &str,
    limit: u32,
    search: Option<&str>,
) -> anyhow::Result<Vec<WorkspaceAuditEntry>> {
    let calls = agent.tools.recent(Some(i64::from(limit))).await?;
    let mut out = Vec::new();

    for (index, call) in calls.into_iter().enumerate() {
        let value = serde_json::to_value(&call)?;
        let tool = value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let params = value.get("parameters").cloned();
        let event = tool.clone();
        let path = params
            .as_ref()
            .and_then(|p| p.get("path"))
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        let status = value
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let error = value
            .get("error")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        let created_at = parse_timestamp_from_tool_call(&value).unwrap_or_else(Utc::now);
        let fallback_id = format!("wsa-{}-{index}", created_at.timestamp_micros());

        let entry = WorkspaceAuditEntry {
            id: value
                .get("id")
                .and_then(serde_json::Value::as_i64)
                .map(|v| format!("wsa-{v}"))
                .unwrap_or(fallback_id),
            session_id: session_id.to_string(),
            workspace_id: workspace_id(session_id),
            source: "file".to_string(),
            event,
            path,
            tool,
            status,
            params,
            result: value.get("result").cloned(),
            error,
            created_at,
        };

        if let Some(query) = search {
            let hay = serde_json::to_string(&entry)?;
            if !hay
                .to_ascii_lowercase()
                .contains(&query.to_ascii_lowercase())
            {
                continue;
            }
        }
        out.push(entry);
    }

    Ok(out)
}

pub async fn active_session_file_audit(
    project_root: &Path,
    limit: u32,
    search: Option<&str>,
) -> anyhow::Result<Vec<WorkspaceAuditEntry>> {
    let agent = open_active_workspace(project_root).await?;
    let session_id: Option<String> = agent.kv.get("session_id").await?;
    let session_id = session_id.ok_or_else(|| anyhow::anyhow!("workspace missing session_id"))?;
    file_audit_from_agent(&agent, &session_id, limit, search).await
}

async fn open_active_workspace(project_root: &Path) -> anyhow::Result<AgentFS> {
    let dir = project_root.join(".zenith").join("workspaces");
    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .with_context(|| format!("read workspace dir {}", dir.to_string_lossy()))?;

    let mut candidates = Vec::<(std::time::SystemTime, PathBuf)>::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("db") {
            continue;
        }

        let modified = tokio::fs::metadata(&path)
            .await
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        candidates.push((modified, path));
    }

    candidates.sort_by_key(|(modified, path)| (*modified, path.clone()));
    let mut entries = candidates
        .into_iter()
        .map(|(_, path)| path)
        .collect::<Vec<_>>();
    let last = entries
        .pop()
        .ok_or_else(|| anyhow::anyhow!("no workspace db files found"))?;
    let path = last
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("invalid workspace path"))?;
    AgentFS::open(AgentFSOptions::with_path(path))
        .await
        .context("open active workspace")
}

async fn open_session_workspace(project_root: &Path, session_id: &str) -> anyhow::Result<AgentFS> {
    let db_path = persistent_workspace_db_path(project_root, session_id).await?;
    let path = db_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("invalid workspace path"))?;
    AgentFS::open(AgentFSOptions::with_path(path))
        .await
        .context("open session workspace")
}

async fn persistent_workspace_db_path(
    project_root: &Path,
    session_id: &str,
) -> anyhow::Result<PathBuf> {
    validate_session_id(session_id)?;
    let dir = project_root.join(".zenith").join("workspaces");
    tokio::fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("create workspace dir {}", dir.to_string_lossy()))?;
    Ok(dir.join(format!("{session_id}.db")))
}

fn validate_session_id(session_id: &str) -> anyhow::Result<()> {
    if session_id.is_empty() {
        anyhow::bail!("invalid session_id: cannot be empty");
    }
    if session_id.contains("..") || session_id.contains('/') || session_id.contains('\\') {
        anyhow::bail!("invalid session_id: path separators are not allowed");
    }
    if !session_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!("invalid session_id: only [A-Za-z0-9_-] are allowed");
    }
    Ok(())
}

fn workspace_id(session_id: &str) -> String {
    format!("ws-{session_id}")
}

fn now_epoch_secs() -> i64 {
    Utc::now().timestamp()
}

fn parse_timestamp_from_tool_call(value: &serde_json::Value) -> Option<DateTime<Utc>> {
    if let Some(ts) = value.get("started_at").and_then(serde_json::Value::as_i64) {
        return DateTime::from_timestamp(ts, 0);
    }
    if let Some(ts) = value.get("ended_at").and_then(serde_json::Value::as_i64) {
        return DateTime::from_timestamp(ts, 0);
    }
    None
}
