#![allow(clippy::all)]

//! # Spike 0.7: AgentFS SDK Validation
//!
//! Validates that the `agentfs-sdk` crate (v0.6.0, from crates.io) works for zenith's
//! workspace isolation and session management needs:
//!
//! - **Crate availability**: `agentfs-sdk = "0.6"` installs from crates.io (no git dep needed)
//! - **Open/create**: `AgentFS::open()` with persistent and ephemeral modes
//! - **KV store**: `kv.set()`, `kv.get()`, `kv.delete()`, `kv.keys()` — for session metadata
//! - **Filesystem**: `fs.mkdir()`, `fs.create_file()` + `fs.pwrite()`, `fs.read_file()`,
//!   `fs.stat()`, `fs.remove()` — for workspace isolation during package indexing
//! - **Tool tracking**: `tools.start()` + `tools.success()`, `tools.record()`,
//!   `tools.recent()`, `tools.stats()` — for audit trail of agent operations
//! - **Ephemeral mode**: In-memory databases for tests and isolated spike runs
//!
//! ## Validates
//!
//! AgentFS compiles from crates.io and works — blocks Phase 7. Decision: proceed with
//! `agentfs-sdk`, fallback (task 0.10) not needed.
//!
//! ## Crate Name Confusion
//!
//! **Important**: The Turso docs at `docs.turso.tech/agentfs/sdk/rust` say `agentfs = "0.1"`,
//! but the actual Turso SDK on crates.io is published as **`agentfs-sdk`** (v0.6.0, by penberg).
//!
//! The `agentfs` crate on crates.io (v0.2.0) is by a different author (cryptopatrick) and is
//! a completely separate project.
//!
//! ## API Mismatch with Turso Docs
//!
//! The Turso documentation describes a high-level API:
//!
//! ```text
//! agent.fs.write_file("/file.txt", b"data").await?;   // Doesn't exist in 0.6.0
//! agent.fs.rm("/file.txt").await?;                     // Doesn't exist in 0.6.0
//! agent.fs.exists("/file.txt").await?;                 // Doesn't exist in 0.6.0
//! agent.kv.set("key", "value").await?;                 // Actually takes &V reference
//! agent.tools.record(ToolCall { ... }).await?;         // Actually takes positional args
//! ```
//!
//! The actual `agentfs-sdk` 0.6.0 API is lower-level and POSIX-oriented:
//!
//! ```text
//! agent.fs.create_file(path, mode, uid, gid).await?;   // Returns (Stats, BoxedFile)
//! agent.fs.pwrite(path, offset, data).await?;           // Write data at offset
//! agent.fs.read_file(path).await?;                      // Returns Option<Vec<u8>>
//! agent.fs.remove(path).await?;                         // Delete file/dir
//! agent.fs.stat(path).await?;                           // Returns Option<Stats>
//! agent.fs.mkdir(path, uid, gid).await?;                // Requires uid/gid
//! agent.kv.set(key, &value).await?;                     // Takes &V reference
//! agent.tools.record(name, started_at, ended_at, params, result, error).await?;
//! ```
//!
//! This spike validates the **actual** API, not the documented one. A thin wrapper layer
//! in zenith may be desirable to provide the simpler `write_file`/`read_file` pattern.
//!
//! ## Dependencies
//!
//! `agentfs-sdk` depends on `turso ^0.4.4` (Limbo-based SQLite). This coexists with
//! our `libsql` 0.9.29 dependency — they are separate database engines:
//! - `libsql`: zenith's own state (Turso Cloud sync)
//! - `turso` (via `agentfs-sdk`): AgentFS's internal storage
//!
//! ## Zenith Usage Plan
//!
//! 1. **Package indexing workspaces**: Each `zen install` creates an ephemeral AgentFS
//!    for clone → parse → index. Virtual filesystem isolates temp files.
//!
//! 2. **Session workspaces**: Each `zen session start` creates a persistent AgentFS
//!    (keyed by session ID). File-level audit via tool tracking.
//!
//! 3. **KV store for session metadata**: Session goal, status, snapshot data as KV pairs.
//!
//! 4. **Tool call tracking for audit**: Record significant operations with timing + I/O.

use agentfs_sdk::filesystem::DEFAULT_FILE_MODE;
use agentfs_sdk::{AgentFS, AgentFSOptions};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

/// Helper: current time as unix epoch seconds (i64), matching AgentFS tool call format.
fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Helper: create an ephemeral (in-memory) AgentFS instance for tests.
async fn ephemeral_agent() -> AgentFS {
    AgentFS::open(AgentFSOptions::ephemeral())
        .await
        .expect("failed to create ephemeral AgentFS")
}

// ---------------------------------------------------------------------------
// Spike tests — all use multi_thread because agentfs-sdk uses turso internally
// which may require it for background tasks.
// ---------------------------------------------------------------------------

/// Verify that AgentFS opens in ephemeral (in-memory) mode.
#[tokio::test(flavor = "multi_thread")]
async fn spike_agentfs_ephemeral_opens() {
    let agent = ephemeral_agent().await;

    // Smoke test: KV set + get to prove the agent is alive
    let value = "hello".to_string();
    agent.kv.set("test", &value).await.unwrap();
    let val: Option<String> = agent.kv.get("test").await.unwrap();
    assert_eq!(val, Some("hello".to_string()));
}

/// Verify that AgentFS opens in persistent mode with a path.
/// Creates a database file on disk.
#[tokio::test(flavor = "multi_thread")]
async fn spike_agentfs_persistent_opens() {
    let dir = TempDir::new().unwrap();

    // Use with_path to control where the DB file goes
    let db_path = dir.path().join("spike-session.db");
    let agent = AgentFS::open(AgentFSOptions::with_path(db_path.to_str().unwrap()))
        .await
        .expect("failed to create persistent AgentFS");

    let goal = "validate agentfs".to_string();
    agent.kv.set("session:goal", &goal).await.unwrap();
    let retrieved: Option<String> = agent.kv.get("session:goal").await.unwrap();
    assert_eq!(retrieved, Some("validate agentfs".to_string()));

    // Verify the database file was created
    assert!(
        db_path.exists(),
        "persistent AgentFS should create db file at: {}",
        db_path.display()
    );
}

/// Verify KV store CRUD operations — zenith uses KV for session metadata.
#[tokio::test(flavor = "multi_thread")]
async fn spike_agentfs_kv_crud() {
    let agent = ephemeral_agent().await;

    // Set various types (all via &V reference)
    let s = "hello world".to_string();
    let n = 42i64;
    let b = true;

    agent.kv.set("string_key", &s).await.unwrap();
    agent.kv.set("int_key", &n).await.unwrap();
    agent.kv.set("bool_key", &b).await.unwrap();

    // Get with type inference
    let got_s: Option<String> = agent.kv.get("string_key").await.unwrap();
    assert_eq!(got_s, Some("hello world".to_string()));

    let got_n: Option<i64> = agent.kv.get("int_key").await.unwrap();
    assert_eq!(got_n, Some(42));

    let got_b: Option<bool> = agent.kv.get("bool_key").await.unwrap();
    assert_eq!(got_b, Some(true));

    // Get missing key
    let missing: Option<String> = agent.kv.get("nonexistent").await.unwrap();
    assert_eq!(missing, None);

    // Delete
    agent.kv.delete("string_key").await.unwrap();
    let deleted: Option<String> = agent.kv.get("string_key").await.unwrap();
    assert_eq!(deleted, None);

    // Overwrite
    let new_n = 99i64;
    agent.kv.set("int_key", &new_n).await.unwrap();
    let updated: Option<i64> = agent.kv.get("int_key").await.unwrap();
    assert_eq!(updated, Some(99));

    // List keys
    let keys = agent.kv.keys().await.unwrap();
    assert!(keys.contains(&"int_key".to_string()));
    assert!(keys.contains(&"bool_key".to_string()));
    assert!(
        !keys.contains(&"string_key".to_string()),
        "deleted key should not appear"
    );
}

/// Verify KV store with structured data (serde serialization) — zenith stores
/// session snapshots and metadata as structured JSON values.
#[tokio::test(flavor = "multi_thread")]
async fn spike_agentfs_kv_structured_data() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct SessionMeta {
        goal: String,
        status: String,
        findings_count: u32,
        tags: Vec<String>,
    }

    let agent = ephemeral_agent().await;

    let meta = SessionMeta {
        goal: "Evaluate HTTP clients for the project".to_string(),
        status: "active".to_string(),
        findings_count: 3,
        tags: vec!["research".to_string(), "http".to_string()],
    };

    agent
        .kv
        .set("session:ses-abc123:meta", &meta)
        .await
        .unwrap();

    let retrieved: Option<SessionMeta> = agent.kv.get("session:ses-abc123:meta").await.unwrap();
    assert_eq!(retrieved, Some(meta));
}

/// Verify filesystem operations — zenith uses AgentFS filesystem for workspace
/// isolation during package indexing (clone → parse → cleanup).
///
/// The actual API is POSIX-level: mkdir(path, uid, gid), create_file(path, mode,
/// uid, gid) + pwrite(), read_file() returns Option<Vec<u8>>, stat() instead of
/// exists(), remove() instead of rm().
#[tokio::test(flavor = "multi_thread")]
async fn spike_agentfs_filesystem_ops() {
    let agent = ephemeral_agent().await;

    // Create directory structure (uid=0, gid=0 for tests)
    agent.fs.mkdir("/workspace", 0, 0).await.unwrap();
    agent.fs.mkdir("/workspace/src", 0, 0).await.unwrap();

    // Create and write a file: create_file() + pwrite()
    let readme_content = b"# Test Package\nA test.";
    agent
        .fs
        .create_file("/workspace/README.md", DEFAULT_FILE_MODE, 0, 0)
        .await
        .unwrap();
    agent
        .fs
        .pwrite("/workspace/README.md", 0, readme_content)
        .await
        .unwrap();

    let lib_content = b"pub fn hello() -> &'static str { \"hello\" }";
    agent
        .fs
        .create_file("/workspace/src/lib.rs", DEFAULT_FILE_MODE, 0, 0)
        .await
        .unwrap();
    agent
        .fs
        .pwrite("/workspace/src/lib.rs", 0, lib_content)
        .await
        .unwrap();

    // Read files back
    let readme = agent.fs.read_file("/workspace/README.md").await.unwrap();
    assert!(
        readme.is_some(),
        "read_file should return Some for existing file"
    );
    assert_eq!(
        String::from_utf8(readme.unwrap()).unwrap(),
        "# Test Package\nA test."
    );

    let lib = agent.fs.read_file("/workspace/src/lib.rs").await.unwrap();
    assert!(
        String::from_utf8(lib.unwrap())
            .unwrap()
            .contains("pub fn hello")
    );

    // Check existence via stat()
    let readme_stat = agent.fs.stat("/workspace/README.md").await.unwrap();
    assert!(
        readme_stat.is_some(),
        "stat should return Some for existing file"
    );

    let missing_stat = agent.fs.stat("/workspace/nonexistent.rs").await.unwrap();
    assert!(
        missing_stat.is_none(),
        "stat should return None for missing file"
    );

    // Read missing file
    let missing = agent
        .fs
        .read_file("/workspace/nonexistent.rs")
        .await
        .unwrap();
    assert!(
        missing.is_none(),
        "read_file should return None for missing file"
    );

    // Remove file
    agent.fs.remove("/workspace/README.md").await.unwrap();
    let removed_stat = agent.fs.stat("/workspace/README.md").await.unwrap();
    assert!(
        removed_stat.is_none(),
        "stat should return None after remove"
    );
}

/// Verify tool call tracking — zenith uses this for audit trail of agent operations.
///
/// AgentFS tools API has two patterns:
/// 1. start() + success()/error() — for ongoing operations
/// 2. record() — for completed operations (insert-only)
#[tokio::test(flavor = "multi_thread")]
async fn spike_agentfs_tool_tracking() {
    let agent = ephemeral_agent().await;

    // Pattern 1: start() + success() for ongoing operations
    let id = agent
        .tools
        .start(
            "zen_install",
            Some(serde_json::json!({
                "package": "tokio",
                "ecosystem": "rust",
                "version": "1.40.0"
            })),
        )
        .await
        .unwrap();

    assert!(id > 0, "start() should return a positive ID");

    // Simulate some work
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Mark as successful with result
    agent
        .tools
        .success(
            id,
            Some(serde_json::json!({
                "symbols_extracted": 1580,
                "doc_chunks": 42,
                "success": true
            })),
        )
        .await
        .unwrap();

    // Verify the recorded tool call
    let call = agent.tools.get(id).await.unwrap();
    assert!(call.is_some(), "get() should return the recorded tool call");
    let call = call.unwrap();
    assert_eq!(call.name, "zen_install");
    assert_eq!(call.status, agentfs_sdk::ToolCallStatus::Success);
    assert!(call.parameters.is_some());
    assert!(call.result.is_some());

    // Pattern 2: record() for completed operations (insert-only)
    let started = now_epoch_secs();
    let completed = started + 1; // 1 second later
    let id2 = agent
        .tools
        .record(
            "zen_search",
            started,
            completed,
            Some(serde_json::json!({"query": "async spawn", "package": "tokio"})),
            Some(serde_json::json!({"results_count": 5, "top_result": "spawn()"})),
            None, // no error
        )
        .await
        .unwrap();

    assert!(
        id2 > id,
        "record() should return an ID greater than previous"
    );

    // List recent tool calls
    let recent = agent.tools.recent(Some(10)).await.unwrap();
    assert!(
        recent.len() >= 2,
        "should have at least 2 tool calls, got {}",
        recent.len()
    );

    // Verify names are present
    let names: Vec<&str> = recent.iter().map(|c| c.name.as_str()).collect();
    assert!(
        names.contains(&"zen_install"),
        "recent should contain zen_install: {names:?}"
    );
    assert!(
        names.contains(&"zen_search"),
        "recent should contain zen_search: {names:?}"
    );

    // Pattern 3: start() + error() for failed operations
    let id3 = agent
        .tools
        .start(
            "zen_install_fail",
            Some(serde_json::json!({"package": "nonexistent"})),
        )
        .await
        .unwrap();

    agent
        .tools
        .error(id3, "package not found on crates.io")
        .await
        .unwrap();

    let failed = agent.tools.get(id3).await.unwrap().unwrap();
    assert_eq!(failed.status, agentfs_sdk::ToolCallStatus::Error);
    assert!(failed.error.is_some());
    assert_eq!(failed.error.unwrap(), "package not found on crates.io");
}

/// Verify tool call statistics — zenith can use this for session summaries.
#[tokio::test(flavor = "multi_thread")]
async fn spike_agentfs_tool_stats() {
    let agent = ephemeral_agent().await;

    // Record several successful tool calls
    let now = now_epoch_secs();
    for i in 0..5 {
        agent
            .tools
            .record(
                "zen_search",
                now + i,
                now + i + 1,
                Some(serde_json::json!({"query": format!("query_{i}")})),
                Some(serde_json::json!({"results": i})),
                None,
            )
            .await
            .unwrap();
    }

    // Record some failures
    for i in 0..2 {
        agent
            .tools
            .record(
                "zen_search",
                now + 10 + i,
                now + 10 + i + 1,
                Some(serde_json::json!({"query": format!("bad_query_{i}")})),
                None,
                Some("timeout"),
            )
            .await
            .unwrap();
    }

    // Record a different tool
    agent
        .tools
        .record(
            "zen_install",
            now,
            now + 5,
            None,
            Some(serde_json::json!({"ok": true})),
            None,
        )
        .await
        .unwrap();

    // Get stats for a specific tool
    let search_stats = agent.tools.stats_for("zen_search").await.unwrap();
    assert!(search_stats.is_some(), "should have stats for zen_search");
    let search_stats = search_stats.unwrap();
    assert_eq!(search_stats.total_calls, 7); // 5 success + 2 error
    assert_eq!(search_stats.successful, 5);
    assert_eq!(search_stats.failed, 2);

    // Get all stats
    let all_stats = agent.tools.stats().await.unwrap();
    assert_eq!(all_stats.len(), 2, "should have stats for 2 tools");
    let tool_names: Vec<&str> = all_stats.iter().map(|s| s.name.as_str()).collect();
    assert!(tool_names.contains(&"zen_search"));
    assert!(tool_names.contains(&"zen_install"));
}

/// End-to-end spike: simulate the package indexing workspace pattern.
/// Create workspace → write files → "parse" → record tool call → cleanup.
#[tokio::test(flavor = "multi_thread")]
async fn spike_agentfs_indexing_workspace_pattern() {
    let agent = ephemeral_agent().await;

    // 1. Create workspace for package indexing
    agent.fs.mkdir("/index-tokio-1.40.0", 0, 0).await.unwrap();
    agent
        .fs
        .mkdir("/index-tokio-1.40.0/src", 0, 0)
        .await
        .unwrap();

    // 2. "Clone" — write source files into workspace
    agent
        .fs
        .create_file("/index-tokio-1.40.0/src/lib.rs", DEFAULT_FILE_MODE, 0, 0)
        .await
        .unwrap();
    agent
        .fs
        .pwrite(
            "/index-tokio-1.40.0/src/lib.rs",
            0,
            b"pub async fn spawn<F>(future: F) -> JoinHandle<F::Output> { todo!() }",
        )
        .await
        .unwrap();

    agent
        .fs
        .create_file("/index-tokio-1.40.0/README.md", DEFAULT_FILE_MODE, 0, 0)
        .await
        .unwrap();
    agent
        .fs
        .pwrite(
            "/index-tokio-1.40.0/README.md",
            0,
            b"# Tokio\nAn async runtime for Rust.",
        )
        .await
        .unwrap();

    // 3. "Parse" — read files back (simulating tree-sitter parse)
    let source = agent
        .fs
        .read_file("/index-tokio-1.40.0/src/lib.rs")
        .await
        .unwrap()
        .expect("source file should exist");
    let source_str = String::from_utf8(source).unwrap();
    assert!(source_str.contains("spawn"));

    // 4. Store indexing metadata in KV
    let status = "completed".to_string();
    let symbols = 1580u32;
    agent
        .kv
        .set("index:tokio:1.40.0:status", &status)
        .await
        .unwrap();
    agent
        .kv
        .set("index:tokio:1.40.0:symbols", &symbols)
        .await
        .unwrap();

    // 5. Record the indexing operation as a tool call
    let started = now_epoch_secs();
    let completed = started + 3;
    agent
        .tools
        .record(
            "index_package",
            started,
            completed,
            Some(serde_json::json!({"package": "tokio", "version": "1.40.0"})),
            Some(serde_json::json!({"symbols": 1580, "chunks": 42})),
            None,
        )
        .await
        .unwrap();

    // 6. "Cleanup" — remove workspace files
    agent
        .fs
        .remove("/index-tokio-1.40.0/src/lib.rs")
        .await
        .unwrap();
    agent
        .fs
        .remove("/index-tokio-1.40.0/README.md")
        .await
        .unwrap();
    agent.fs.remove("/index-tokio-1.40.0/src").await.unwrap();
    agent.fs.remove("/index-tokio-1.40.0").await.unwrap();

    // Verify cleanup
    let removed = agent.fs.stat("/index-tokio-1.40.0").await.unwrap();
    assert!(removed.is_none(), "workspace directory should be removed");

    // Verify metadata persisted beyond workspace cleanup
    let idx_status: Option<String> = agent.kv.get("index:tokio:1.40.0:status").await.unwrap();
    assert_eq!(idx_status, Some("completed".to_string()));

    // Verify tool call persisted
    let recent = agent.tools.recent(Some(1)).await.unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].name, "index_package");
}
