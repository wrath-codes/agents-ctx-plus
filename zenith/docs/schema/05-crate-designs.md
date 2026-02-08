# Zenith: Crate Design Specifications

**Version**: 2026-02-08 (v2)
**Status**: Design Document
**Purpose**: Per-crate implementation guide with validated patterns, dependencies, module structure, and key types

**Changes from v1**: Switched to `turso` crate (replaces `libsql`), Turso-native ID generation (removed sha2/uuid/base32), added `issues` entity, added AgentFS workspace integration from git, added workspace trait abstraction.

**Changes from v3**: Switched back from `turso` crate (Limbo-based, pre-release) to `libsql` crate (C SQLite fork, stable). The `turso` crate (v0.5.0-pre.8) does not expose FTS support — its tantivy-backed FTS is behind an experimental `index_method` flag that `turso::Builder` doesn't surface. The `libsql` crate provides native FTS5, stable API, and embedded replica support. Plan: switch to `turso` once it stabilizes and exposes FTS.

**Changes from v2**: Replaced direct tree-sitter + individual grammar crates with `ast-grep-core` + `ast-grep-language` for zen-parser. This gives us pattern-based AST matching, jQuery-like traversal, composable matchers, and bundled grammar management for 26 languages via feature flags. Dropped unsupported languages (Zig, Svelte, Astro, Gleam, Mojo, Markdown) from initial scope — these can be added later by implementing ast-grep's `Language` trait. Removed `grammars/` directory (no longer needed). Updated dependency versions to match crates.io actuals.

---

## Table of Contents

1. [Workspace Layout](#1-workspace-layout)
2. [Dependency Graph](#2-dependency-graph)
3. [zen-core](#3-zen-core)
4. [zen-config](#4-zen-config)
5. [zen-db](#5-zen-db)
6. [zen-lake](#6-zen-lake)
7. [zen-parser](#7-zen-parser)
8. [zen-embeddings](#8-zen-embeddings)
9. [zen-registry](#9-zen-registry)
10. [zen-search](#10-zen-search)
11. [zen-cli](#11-zen-cli)
12. [Implementation Order](#12-implementation-order)

---

## 1. Workspace Layout

```
zenith/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── zen-core/              # Types, IDs, errors
│   ├── zen-config/            # Configuration (figment)
│   ├── zen-db/                # Turso/libSQL operations
│   ├── zen-lake/              # DuckDB/DuckLake operations
│   ├── zen-parser/            # ast-grep-based parsing + extraction
│   ├── zen-embeddings/        # fastembed integration
│   ├── zen-registry/          # Package registry HTTP clients
│   ├── zen-search/            # Search orchestration
│   └── zen-cli/               # CLI binary (clap)
└── docs/
```

### Workspace Cargo.toml Pattern

**Validated in**: aether `Cargo.toml`

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/<org>/zenith"

[workspace.dependencies]
# Async
tokio = { version = "1.49", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Database
duckdb = { version = "1.4", features = ["bundled"] }
libsql = "0.9.29"
# NOTE: turso crate (Limbo-based) planned for future switch once FTS is stable.
# turso = "0.5.0-pre.8"

# Workspace isolation (try AgentFS from git first, fallback to manual)
# agentfs = { git = "https://github.com/tursodatabase/agentfs", path = "sdk/rust" }
# NOTE: Commented out until spike 0.7 validates it compiles. Uncomment when ready.

# Embeddings
fastembed = "5.8"

# Parsing (ast-grep replaces direct tree-sitter + individual grammar crates)
ast-grep-core = "0.40"
ast-grep-language = "0.40"

# HTTP
reqwest = { version = "0.13", features = ["json"] }

# CLI
clap = { version = "4.5", features = ["derive"] }

# Config
figment = { version = "0.10", features = ["toml", "env"] }

# Object storage
object_store = { version = "0.13", features = ["aws"] }

# Error handling
thiserror = "2.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Env
dotenvy = "0.15"
dirs = "6.0"

# Testing
pretty_assertions = "1.4"
tempfile = "3.20"
rstest = "0.26"

# Internal
zen-core = { path = "crates/zen-core" }
zen-config = { path = "crates/zen-config" }
zen-db = { path = "crates/zen-db" }
zen-lake = { path = "crates/zen-lake" }
zen-parser = { path = "crates/zen-parser" }
zen-embeddings = { path = "crates/zen-embeddings" }
zen-registry = { path = "crates/zen-registry" }
zen-search = { path = "crates/zen-search" }

[workspace.lints.rust]
unsafe_code = "forbid"
unused_must_use = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

---

## 2. Dependency Graph

```
zen-core (types, IDs, errors)
  │
  ├──► zen-config (configuration)
  │
  ├──► zen-db (Turso operations)
  │      │
  │      └──► zen-core, zen-config
  │
  ├──► zen-lake (DuckDB/DuckLake)
  │      │
  │      └──► zen-core, zen-config, zen-embeddings
  │
  ├──► zen-parser (ast-grep)
│      │
│      └──► zen-core
  │
  ├──► zen-embeddings (fastembed)
  │      │
  │      └──► zen-core
  │
  ├──► zen-registry (HTTP clients)
  │      │
  │      └──► zen-core
  │
  ├──► zen-search (search orchestration)
  │      │
  │      └──► zen-core, zen-db, zen-lake, zen-embeddings
  │
  └──► zen-cli (binary)
         │
         └──► ALL crates
```

---

## 3. zen-core

**Purpose**: Shared types, ID generation, error hierarchy. Every other crate depends on this.

### Dependencies

```toml
[dependencies]
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
thiserror.workspace = true
```

Note: no `sha2`, `uuid`, or `base32` -- IDs are generated by Turso via `hex(randomblob(4))`.

### Module Structure

```
zen-core/src/
├── lib.rs
├── ids.rs              # ID prefixes and formatting (no generation logic)
├── errors.rs           # Error hierarchy
├── entities/
│   ├── mod.rs
│   ├── research.rs     # ResearchItem
│   ├── finding.rs      # Finding, FindingTag
│   ├── hypothesis.rs   # Hypothesis
│   ├── insight.rs      # Insight
│   ├── issue.rs        # Issue (bug, feature, spike, epic, request)
│   ├── task.rs         # Task
│   ├── impl_log.rs     # ImplementationLog
│   ├── compat.rs       # CompatibilityCheck
│   ├── session.rs      # Session, SessionSnapshot
│   ├── project.rs      # ProjectMeta, ProjectDependency
│   ├── link.rs         # EntityLink
│   └── audit.rs        # AuditEntry
└── enums.rs            # Status enums, EntityType, Relation, Action
```

### Key Types

#### ID Generation

IDs are generated by Turso in SQL. The Rust layer only handles prefixes and formatting.

```rust
/// ID prefix constants. The actual random part is generated by Turso:
/// `lower(hex(randomblob(4)))` → 8-char hex string like "a3f8b2c1"
pub const PREFIX_SESSION: &str = "ses";
pub const PREFIX_RESEARCH: &str = "res";
pub const PREFIX_FINDING: &str = "fnd";
pub const PREFIX_HYPOTHESIS: &str = "hyp";
pub const PREFIX_INSIGHT: &str = "ins";
pub const PREFIX_ISSUE: &str = "iss";
pub const PREFIX_TASK: &str = "tsk";
pub const PREFIX_IMPL_LOG: &str = "imp";
pub const PREFIX_COMPAT: &str = "cmp";
pub const PREFIX_LINK: &str = "lnk";
pub const PREFIX_AUDIT: &str = "aud";

/// Format a prefixed ID. Called after Turso generates the random part.
pub fn format_id(prefix: &str, random: &str) -> String {
    format!("{}-{}", prefix, random)
}

/// SQL expression for generating a prefixed ID in an INSERT.
/// Usage: `conn.execute(&gen_id_sql("fnd"), ...)`
pub fn gen_id_sql(prefix: &str) -> String {
    format!("'{}-' || lower(hex(randomblob(4)))", prefix)
}
```

#### Error Hierarchy

**Validated in**: aether `AetherError` pattern

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZenError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Lake error: {0}")]
    Lake(#[from] LakeError),

    #[error("Parser error: {0}")]
    Parser(#[from] ParserError),

    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),

    #[error("Entity not found: {entity_type} {id}")]
    NotFound { entity_type: String, id: String },

    #[error("Invalid state transition: {entity_type} {id} from {from} to {to}")]
    InvalidTransition {
        entity_type: String,
        id: String,
        from: String,
        to: String,
    },

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
```

#### Entity Structs

All entities follow the same pattern: serde-derivable structs with chrono timestamps.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub research_id: Option<String>,
    pub session_id: Option<String>,
    pub content: String,
    pub source: Option<String>,
    pub confidence: Confidence,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisStatus {
    Unverified,
    Analyzing,
    Confirmed,
    Debunked,
    PartiallyConfirmed,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Done,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub issue_type: IssueType,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: IssueStatus,
    pub priority: u8,  // 1 (highest) to 5 (lowest)
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    Bug,
    Feature,
    Spike,
    Epic,
    Request,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    Open,
    InProgress,
    Done,
    Blocked,
    Abandoned,
}

// ... similar patterns for all entities
```

#### Audit Entry

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub session_id: Option<String>,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub action: AuditAction,
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Created,
    Updated,
    StatusChanged,
    Linked,
    Unlinked,
    Tagged,
    Untagged,
    Indexed,
    SessionStart,
    SessionEnd,
    WrapUp,
}
```

### Tests

- ID collision resistance: generate 10,000 IDs, assert uniqueness
- ID prefix correctness: each entity type produces correct prefix
- Serde roundtrip: every entity serializes/deserializes correctly
- Status transition validation: only valid transitions allowed

---

## 4. zen-config

**Purpose**: Layered configuration loading from env vars, TOML files, and defaults.

**Validated in**: aether `aether-config` (figment with TOML + env layering, fully tested)

### Dependencies

```toml
[dependencies]
zen-core.workspace = true
figment.workspace = true
serde.workspace = true
dirs.workspace = true
dotenvy.workspace = true
thiserror.workspace = true
```

### Module Structure

```
zen-config/src/
├── lib.rs              # ZenConfig struct, load(), from_env()
├── turso.rs            # TursoConfig (url, token, db_name)
├── motherduck.rs       # MotherDuckConfig (token, db_name)
├── r2.rs               # R2Config (account_id, access_key, secret, bucket)
└── general.rs          # GeneralConfig (auto_commit, default_ecosystem, limit)
```

### Key Types

**Pattern from**: aether `AetherConfig`

```rust
use figment::{Figment, providers::{Env, Format, Toml, Serialized}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenConfig {
    #[serde(default)]
    pub turso: TursoConfig,
    #[serde(default)]
    pub motherduck: MotherDuckConfig,
    #[serde(default)]
    pub r2: R2Config,
    #[serde(default)]
    pub general: GeneralConfig,
}

impl ZenConfig {
    /// Load config with layered precedence:
    /// env vars > ./zenith.toml > ~/.config/zenith/config.toml > defaults
    pub fn load() -> Result<Self, ConfigError> {
        let figment = Self::figment();
        figment.extract().map_err(ConfigError::from)
    }

    fn figment() -> Figment {
        let mut figment = Figment::new()
            .merge(Serialized::defaults(ZenConfig::default()));

        // Global config
        if let Some(config_dir) = dirs::config_dir() {
            let global_path = config_dir.join("zenith").join("config.toml");
            if global_path.exists() {
                figment = figment.merge(Toml::file(global_path));
            }
        }

        // Project-local config
        let local_path = std::path::Path::new(".zenith/config.toml");
        if local_path.exists() {
            figment = figment.merge(Toml::file(local_path));
        }

        // Env vars (ZENITH_ prefix)
        figment.merge(Env::prefixed("ZENITH_").split("__"))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TursoConfig {
    pub url: Option<String>,
    pub auth_token: Option<String>,
    pub db_name: Option<String>,
    pub sync_interval_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct R2Config {
    pub account_id: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub bucket_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MotherDuckConfig {
    pub access_token: Option<String>,
    pub db_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub auto_commit: bool,
    pub default_ecosystem: Option<String>,
    pub default_limit: u32,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            auto_commit: false,
            default_ecosystem: None,
            default_limit: 20,
        }
    }
}
```

### Tests

**Pattern from**: aether `config/tests/toml_loading.rs`

- Default config loads without any files
- TOML file loading (use tempfile)
- Env var override (set `ZENITH_TURSO__URL`)
- Layered precedence: env > local TOML > global TOML > defaults
- `is_configured()` checks for each sub-config

---

## 5. zen-db

**Purpose**: All Turso operations -- CRUD for every entity, FTS5 queries, audit trail, session management.

**Validated in**: klaw-effect-tracker `db/schemas/docs.ts` and `db/schemas/findings.ts` (schema init, FTS5, triggers), aether Turso patterns

### Dependencies

```toml
[dependencies]
zen-core.workspace = true
zen-config.workspace = true
libsql.workspace = true
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
tokio.workspace = true
thiserror.workspace = true
anyhow.workspace = true
tracing.workspace = true
```

### Module Structure

```
zen-db/src/
├── lib.rs              # ZenDb struct, connection management
├── migrations.rs       # Schema creation + migration runner
├── repos/
│   ├── mod.rs
│   ├── research.rs     # ResearchRepo (CRUD + FTS)
│   ├── finding.rs      # FindingRepo (CRUD + tags + FTS)
│   ├── hypothesis.rs   # HypothesisRepo (CRUD + status transitions)
│   ├── insight.rs      # InsightRepo (CRUD + FTS)
│   ├── issue.rs        # IssueRepo (CRUD + FTS + parent/child)
│   ├── task.rs         # TaskRepo (CRUD + FTS)
│   ├── impl_log.rs     # ImplLogRepo (CRUD)
│   ├── compat.rs       # CompatRepo (CRUD)
│   ├── link.rs         # LinkRepo (create, delete, query by entity)
│   ├── audit.rs        # AuditRepo (append, query with filters)
│   ├── session.rs      # SessionRepo (start, end, snapshot)
│   └── project.rs      # ProjectRepo (meta, dependencies)
└── sync.rs             # libSQL cloud sync via Turso (wrap-up only)
```

### Key Patterns

#### Connection Management

**Validated in**: spike 0.2 (`libsql` crate v0.9.29), aether Turso embedded replica pattern

```rust
use libsql::Builder;

pub struct ZenDb {
    db: libsql::Database,
    conn: libsql::Connection,
}

impl ZenDb {
    /// Open local-only database (no cloud sync)
    pub async fn open_local(path: &str) -> Result<Self, DatabaseError> {
        let db = Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        let zen_db = Self { db, conn };
        zen_db.run_migrations().await?;
        Ok(zen_db)
    }

    /// Open with Turso cloud sync (embedded replica)
    pub async fn open_synced(
        local_path: &str,
        remote_url: &str,
        auth_token: &str,
    ) -> Result<Self, DatabaseError> {
        let db = Builder::new_remote_replica(local_path, remote_url.to_string(), auth_token.to_string())
            .build()
            .await?;
        let conn = db.connect()?;
        let zen_db = Self { db, conn };
        zen_db.run_migrations().await?;
        Ok(zen_db)
    }

    /// Sync to cloud (called only at wrap-up)
    pub async fn sync(&self) -> Result<(), DatabaseError> {
        self.db.sync().await?;
        Ok(())
    }
}

// ID generation helper -- uses libSQL/SQLite's randomblob in SQL
impl ZenDb {
    /// Generate a prefixed ID via libSQL. Returns e.g., "fnd-a3f8b2c1"
    pub async fn generate_id(&self, prefix: &str) -> Result<String, DatabaseError> {
        let mut rows = self.conn.query(
            &format!("SELECT '{}-' || lower(hex(randomblob(4)))", prefix),
            (),
        ).await?;
        let row = rows.next().await?.ok_or(DatabaseError::NoResult)?;
        Ok(row.get::<String>(0)?)
    }
}
```

#### Repository Pattern

Each entity gets a repo module with standardized CRUD + entity-specific methods.

```rust
// repos/finding.rs
impl ZenDb {
    pub async fn create_finding(&self, finding: &Finding) -> Result<(), DatabaseError> {
        self.conn.execute(
            "INSERT INTO findings (id, research_id, session_id, content, source, confidence, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            [&finding.id, &finding.research_id, &finding.session_id,
             &finding.content, &finding.source, &finding.confidence.as_str(),
             &finding.created_at.to_rfc3339(), &finding.updated_at.to_rfc3339()],
        ).await?;

        // Write audit entry
        self.append_audit(AuditEntry::new(
            EntityType::Finding, &finding.id,
            AuditAction::Created, finding.session_id.as_deref(),
            None,
        )).await?;

        Ok(())
    }

    pub async fn tag_finding(&self, finding_id: &str, tag: &str) -> Result<(), DatabaseError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO finding_tags (finding_id, tag) VALUES (?1, ?2)",
            [finding_id, tag],
        ).await?;

        self.append_audit(AuditEntry::new(
            EntityType::Finding, finding_id,
            AuditAction::Tagged, None,
            Some(serde_json::json!({"tag": tag})),
        )).await?;

        Ok(())
    }

    pub async fn search_findings(&self, query: &str, limit: u32) -> Result<Vec<Finding>, DatabaseError> {
        let rows = self.conn.query(
            "SELECT f.* FROM findings_fts
             JOIN findings f ON f.rowid = findings_fts.rowid
             WHERE findings_fts MATCH ?1
             ORDER BY rank LIMIT ?2",
            [query, &limit.to_string()],
        ).await?;

        // ... map rows to Finding structs
    }
}
```

#### Migration Runner

**Pattern from**: klaw-effect-tracker `schemas/docs.ts` (lazy init with `initialized` flag)

```rust
impl ZenDb {
    async fn run_migrations(&self) -> Result<(), DatabaseError> {
        // All table creation, FTS5 virtual tables, indexes, and triggers
        // from 01-turso-data-model.md executed in a single transaction
        self.conn.execute_batch(include_str!("../migrations/001_initial.sql")).await?;
        Ok(())
    }
}
```

The SQL file is embedded via `include_str!` from `crates/zen-db/migrations/001_initial.sql` containing the full schema from `01-turso-data-model.md`.

### Tests

- CRUD roundtrip for every entity
- FTS5 search with porter stemming ("spawning" matches "spawn")
- Tag add/remove with audit trail verification
- Hypothesis status transitions (valid and invalid)
- Session lifecycle (start → wrap-up, orphan detection)
- Entity links creation and bidirectional query
- Audit trail filtering by entity, action, session

---

## 6. zen-lake

**Purpose**: DuckDB/DuckLake operations -- package indexing storage, vector queries, parquet management.

**Validated in**: aether `aether-storage/src/bin/test_r2_ducklake.rs` (DuckDB + R2 + DuckLake + MotherDuck full spike)

### Dependencies

```toml
[dependencies]
zen-core.workspace = true
zen-config.workspace = true
zen-embeddings.workspace = true
duckdb.workspace = true
object_store.workspace = true
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
thiserror.workspace = true
anyhow.workspace = true
tracing.workspace = true
tokio.workspace = true
```

### Module Structure

```
zen-lake/src/
├── lib.rs              # ZenLake struct, connection modes
├── setup.rs            # Extension loading, DuckLake init, R2 secret creation
├── indexing.rs          # Write api_symbols + doc_chunks to DuckLake
├── queries.rs           # Vector search, filtered queries, cross-package
└── packages.rs          # indexed_packages CRUD
```

### Key Patterns

#### Connection Modes

**Validated in**: aether storage spike (three modes tested)

```rust
use duckdb::Connection;

pub struct ZenLake {
    conn: Connection,
    mode: LakeMode,
}

pub enum LakeMode {
    /// Local DuckDB file, no cloud
    Local,
    /// MotherDuck catalog + R2 storage
    Cloud {
        motherduck_token: String,
        r2_config: R2Config,
    },
}

impl ZenLake {
    pub fn open_local(path: &str) -> Result<Self, LakeError> {
        let conn = Connection::open(path)?;
        let lake = Self { conn, mode: LakeMode::Local };
        lake.load_extensions()?;
        lake.create_tables()?;
        Ok(lake)
    }

    pub fn open_cloud(config: &ZenConfig) -> Result<Self, LakeError> {
        let md = config.motherduck.as_ref()
            .ok_or(LakeError::MissingConfig("motherduck"))?;
        let conn_str = format!("md:?motherduck_token={}", md.access_token.as_ref().unwrap());
        let conn = Connection::open(&conn_str)?;

        let lake = Self {
            conn,
            mode: LakeMode::Cloud {
                motherduck_token: md.access_token.clone().unwrap(),
                r2_config: config.r2.clone().unwrap(),
            },
        };
        lake.load_extensions()?;
        lake.setup_r2_secret()?;
        lake.setup_ducklake()?;
        Ok(lake)
    }

    fn load_extensions(&self) -> Result<(), LakeError> {
        self.conn.execute_batch("
            INSTALL vss; LOAD vss;
            INSTALL httpfs; LOAD httpfs;
        ")?;
        Ok(())
    }

    fn setup_r2_secret(&self) -> Result<(), LakeError> {
        // Pattern from aether test_r2_ducklake.rs
        if let LakeMode::Cloud { r2_config, .. } = &self.mode {
            self.conn.execute(&format!("
                CREATE SECRET IF NOT EXISTS r2_zenith IN MOTHERDUCK (
                    TYPE s3,
                    KEY_ID '{}',
                    SECRET '{}',
                    ENDPOINT '{}.r2.cloudflarestorage.com',
                    URL_STYLE 'path'
                )", r2_config.access_key_id, r2_config.secret_access_key, r2_config.account_id))?;
        }
        Ok(())
    }
}
```

#### Indexing

```rust
impl ZenLake {
    pub fn store_symbols(&self, symbols: &[ApiSymbol]) -> Result<(), LakeError> {
        // Batch insert into api_symbols table
        let mut appender = self.conn.appender("api_symbols")?;
        for sym in symbols {
            appender.append_row([
                &sym.id, &sym.ecosystem, &sym.package, &sym.version,
                &sym.file_path, &sym.kind, &sym.name, &sym.signature,
                // ... all fields including embedding
            ])?;
        }
        appender.flush()?;
        Ok(())
    }

    pub fn store_doc_chunks(&self, chunks: &[DocChunk]) -> Result<(), LakeError> {
        // Same appender pattern
    }
}
```

### Tests

- Local mode: create tables, insert symbols, query back
- Vector search: insert with embeddings, search by cosine similarity
- Package registration and lookup
- Extension loading verification

---

## 7. zen-parser

**Purpose**: ast-grep-based parsing and API extraction across all 26 built-in languages (rich extractors for 7, generic for 19). This is the richest crate -- it ports the extraction logic from klaw-effect-tracker, using ast-grep's pattern matching and Node traversal API instead of raw tree-sitter.

**Validated in**: klaw-effect-tracker `rust-treesitter.ts` (788 lines) and `python-treesitter.ts` (1044 lines), plus our Go extractors (10 languages)

**Key change (v3)**: Replaced direct tree-sitter + individual grammar crates with `ast-grep-core` + `ast-grep-language`. This gives us:
- Pattern-based AST matching (`fn $NAME($$$PARAMS) -> $RET { $$$ }` syntax)
- jQuery-like Node traversal (`node.find()`, `node.field("name")`, `node.dfs()`)
- Composable matchers (`All`, `Any`, `Not` combinators)
- MetaVariable capture (like regex capture groups for AST nodes)
- 26 built-in language grammars managed via feature flags (no manual grammar version tracking)

**Language scope**: All 26 ast-grep built-in languages are supported for parsing. Extractors are tiered:
- **Rich** (7): Rust, Python, TypeScript, TSX, JavaScript, Go, Elixir -- full `ParsedItem` metadata (signatures, doc comments, generics, visibility, error detection, etc.)
- **Basic** (19): All other built-in languages (C, C++, Java, Ruby, etc.) -- generic kind-based extraction capturing function/class/type definitions with names and signatures
- **Not yet supported**: Zig, Svelte, Astro, Gleam, Mojo, Markdown, TOML -- can be added later via ast-grep's `Language` trait

### Dependencies

```toml
[dependencies]
zen-core.workspace = true
ast-grep-core.workspace = true
ast-grep-language.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

Note: `ast-grep-language` bundles tree-sitter grammars behind feature flags. No need for individual `tree-sitter-*` grammar crates.

### Module Structure

```
zen-parser/src/
├── lib.rs               # Public API: parse_file, extract_api, detect_language
├── parser.rs            # ast-grep wrapper, language detection, SupportLang mapping
├── types.rs             # ParsedItem, SymbolMetadata, DocSections
├── test_files.rs        # IsTestFile, IsTestDir
├── extractors/
│   ├── mod.rs           # Extraction orchestrator (two-tier fallback: ast-grep → regex)
│   ├── generic.rs       # Generic kind-based extractor (works for all 26 languages)
│   ├── rust.rs          # Rust rich extractor (port of klaw rust-treesitter.ts)
│   ├── python.rs        # Python rich extractor (port of klaw python-treesitter.ts)
│   ├── typescript.rs    # TypeScript/JavaScript/TSX rich extractor
│   ├── go.rs            # Go rich extractor
│   └── elixir.rs        # Elixir rich extractor
└── format.rs            # FormatAPIIndex (compressed AGENTS.md style output)
```

### Key Types

**Ported from**: klaw-effect-tracker `ParsedItem` interface (both Rust and Python variants)

```rust
/// Rich symbol representation extracted from source code via ast-grep.
/// This is the core data structure that gets stored in the DuckLake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedItem {
    pub kind: SymbolKind,
    pub name: String,
    pub signature: String,
    pub source: Option<String>,     // Full source up to 50 lines
    pub doc_comment: String,
    pub start_line: u32,
    pub end_line: u32,
    pub visibility: Visibility,
    pub metadata: SymbolMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function, Method, Struct, Enum, Trait, Interface, Class,
    TypeAlias, Const, Static, Macro, Module, Union, Use,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public, PublicCrate, Private, Export, Protected,
}

/// Language-specific metadata. Not every field applies to every language.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolMetadata {
    // Common
    pub is_async: bool,
    pub is_unsafe: bool,
    pub return_type: Option<String>,
    pub generics: Option<String>,
    pub attributes: Vec<String>,
    pub parameters: Vec<String>,

    // Rust-specific
    pub lifetimes: Vec<String>,
    pub where_clause: Option<String>,
    pub trait_name: Option<String>,
    pub for_type: Option<String>,
    pub associated_types: Vec<String>,
    pub abi: Option<String>,
    pub is_pyo3: bool,

    // Enum/Struct members
    pub variants: Vec<String>,
    pub fields: Vec<String>,
    pub methods: Vec<String>,

    // Python-specific
    pub is_generator: bool,
    pub is_property: bool,
    pub is_classmethod: bool,
    pub is_staticmethod: bool,
    pub is_dataclass: bool,
    pub is_pydantic: bool,
    pub is_protocol: bool,
    pub is_enum: bool,
    pub base_classes: Vec<String>,
    pub decorators: Vec<String>,

    // TypeScript-specific
    pub is_exported: bool,
    pub is_default_export: bool,
    pub type_parameters: Option<String>,
    pub implements: Vec<String>,

    // Documentation
    pub doc_sections: DocSections,

    // Error detection
    pub is_error_type: bool,
    pub returns_result: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocSections {
    pub errors: Option<String>,
    pub panics: Option<String>,
    pub safety: Option<String>,
    pub examples: Option<String>,
    pub args: HashMap<String, String>,
    pub returns: Option<String>,
    pub raises: HashMap<String, String>,
    pub yields: Option<String>,
    pub notes: Option<String>,
}
```

### Extraction Orchestrator

**Ported from**: klaw-effect-tracker `extractors/index.ts` (adapted to two-tier with ast-grep)

```rust
use ast_grep_language::{LanguageExt, SupportLang};

/// Extract API from source code. Uses ast-grep pattern matching + Node traversal,
/// falls back to regex for edge cases.
pub fn extract_api(source: &str, language: SupportLang) -> Result<Vec<ParsedItem>, ParserError> {
    // Tier 1: ast-grep pattern matching + Node traversal (preferred)
    match extract_with_ast_grep(source, language) {
        Ok(items) if !items.is_empty() => return Ok(items),
        Ok(_) => tracing::debug!("ast-grep returned no items for {:?}", language),
        Err(e) => tracing::warn!("ast-grep extraction failed for {:?}: {}", language, e),
    }

    // Tier 2: Regex fallback (last resort)
    match extract_with_regex(source, language) {
        Ok(items) => Ok(items),
        Err(e) => {
            tracing::warn!("regex extraction failed for {:?}: {}", language, e);
            Ok(vec![])
        }
    }
}

fn extract_with_ast_grep(source: &str, language: SupportLang) -> Result<Vec<ParsedItem>, ParserError> {
    let root = language.ast_grep(source);
    match language {
        // Rich extractors: full metadata, doc comments, language-specific features
        SupportLang::Rust => extractors::rust::extract(&root),
        SupportLang::Python => extractors::python::extract(&root),
        SupportLang::TypeScript | SupportLang::Tsx | SupportLang::JavaScript => {
            extractors::typescript::extract(&root)
        }
        SupportLang::Go => extractors::go::extract(&root),
        SupportLang::Elixir => extractors::elixir::extract(&root),
        // All other 19 built-in languages: generic kind-based extraction
        _ => extractors::generic::extract(&root, language),
    }
}
```

### Rust Extractor Detail

**Ported from**: klaw-effect-tracker `rust-treesitter.ts` (788 lines). Rewritten to use ast-grep pattern matching + Node traversal.

```rust
// extractors/rust.rs
use ast_grep_core::{AstGrep, Node, matcher::KindMatcher, ops::Any};
use ast_grep_language::SupportLang;

/// Top-level Rust node kinds to extract.
const RUST_ITEM_KINDS: &[&str] = &[
    "function_item", "struct_item", "enum_item", "trait_item",
    "impl_item", "type_item", "mod_item", "const_item", "static_item",
    "macro_definition", "use_declaration", "foreign_mod_item", "union_item",
];

pub fn extract(root: &AstGrep<impl ast_grep_core::Doc>) -> Result<Vec<ParsedItem>, ParserError> {
    let mut items = Vec::new();

    // Use ast-grep KindMatcher + Any combinator to find all top-level items
    let matcher = Any::new(
        RUST_ITEM_KINDS.iter()
            .map(|k| KindMatcher::new(k, SupportLang::Rust))
            .collect()
    );

    for node in root.root().find_all(matcher) {
        if let Some(item) = process_rust_node(&node) {
            items.push(item);
        }
    }
    Ok(items)
}

fn process_rust_node(node: &Node<impl ast_grep_core::Doc>) -> Option<ParsedItem> {
    let kind = match node.kind().as_ref() {
        "function_item" => SymbolKind::Function,
        "struct_item" => SymbolKind::Struct,
        "enum_item" => SymbolKind::Enum,
        "trait_item" => SymbolKind::Trait,
        "impl_item" => return process_impl_item(node),
        "type_item" => SymbolKind::TypeAlias,
        "mod_item" => SymbolKind::Module,
        "const_item" => SymbolKind::Const,
        "static_item" => SymbolKind::Static,
        "macro_definition" => SymbolKind::Macro,
        "union_item" => SymbolKind::Union,
        _ => return None,
    };

    // ast-grep Node API: .field("name") accesses named children by field
    let name = node.field("name")
        .map(|n| n.text().to_string())
        .unwrap_or_default();

    Some(ParsedItem {
        kind,
        name: name.clone(),
        signature: extract_signature(node),
        source: extract_full_source(node, 50),
        doc_comment: extract_doc_comments(node),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: extract_visibility(node),
        metadata: SymbolMetadata {
            is_async: node.text().starts_with("async") ||
                      node.text().starts_with("pub async"),
            is_unsafe: node.text().contains("unsafe"),
            return_type: extract_return_type(node),
            generics: extract_generics(node),
            attributes: extract_attributes(node),
            lifetimes: extract_lifetimes(node),
            where_clause: extract_where_clause(node),
            is_pyo3: is_pyo3_item(node),
            is_error_type: is_error_type(&name, node),
            returns_result: returns_result(node),
            variants: if kind == SymbolKind::Enum {
                extract_enum_variants(node)
            } else { vec![] },
            fields: if kind == SymbolKind::Struct {
                extract_struct_fields(node)
            } else { vec![] },
            doc_sections: parse_doc_sections(&extract_doc_comments(node)),
            ..Default::default()
        },
    })
}

// Helper: extract signature (everything before first { or ;)
fn extract_signature(node: &Node<impl ast_grep_core::Doc>) -> String {
    let text = node.text().to_string();
    let brace = text.find('{');
    let semi = text.find(';');
    let end = match (brace, semi) {
        (Some(b), Some(s)) => b.min(s),
        (Some(b), None) => b,
        (None, Some(s)) => s,
        (None, None) => text.len(),
    };
    text[..end].trim().to_string()
}

// Helper: extract doc comments by walking backward through siblings
fn extract_doc_comments(node: &Node<impl ast_grep_core::Doc>) -> String {
    let mut comments = Vec::new();
    let mut current = node.prev();
    while let Some(sibling) = current {
        let kind = sibling.kind();
        if kind == "line_comment" {
            let text = sibling.text().to_string();
            if text.starts_with("///") || text.starts_with("//!") {
                comments.push(
                    text.trim_start_matches("///")
                        .trim_start_matches("//!")
                        .trim()
                        .to_string()
                );
            } else {
                break;
            }
        } else if kind == "attribute_item" {
            // Skip attributes, keep looking for docs
        } else {
            break;
        }
        current = sibling.prev();
    }
    comments.reverse();
    comments.join("\n")
}

// Helper: detect error types
fn is_error_type(name: &str, node: &Node<impl ast_grep_core::Doc>) -> bool {
    name.ends_with("Error") || {
        let attrs = extract_attributes(node);
        attrs.iter().any(|a| a.contains("Error"))
    }
}
```

### Tests

- Parse and extract from real Rust source (use `include_str!` with test fixtures)
- Parse and extract from real Python source
- Parse and extract from real TypeScript source
- Verify `ParsedItem` metadata fields (async, unsafe, generics, doc comments)
- Two-tier fallback: ast-grep extraction empty triggers regex fallback
- Test file detection for all supported languages
- Signature extraction accuracy (no body leaks)
- Doc comment extraction (///, #[doc], docstrings, JSDoc)
- Error type detection (name pattern, derive(Error))
- impl block processing (inherent vs trait impl)
- ast-grep pattern matching: verify patterns correctly capture metavariables
- ast-grep Node traversal: verify field access, children iteration, sibling walking

---

## 8. zen-embeddings

**Purpose**: fastembed integration for local embedding generation.

**Validated in**: aether `aether-embeddings` (stub), fastembed reference docs

### Dependencies

```toml
[dependencies]
zen-core.workspace = true
fastembed.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

### Module Structure

```
zen-embeddings/src/
├── lib.rs              # EmbeddingEngine struct, embed_batch, embed_single
```

### Key Types

```rust
use fastembed::{TextEmbedding, EmbeddingModel, InitOptions};

pub struct EmbeddingEngine {
    model: TextEmbedding,
}

impl EmbeddingEngine {
    pub fn new() -> Result<Self, EmbeddingError> {
        let model = TextEmbedding::try_new(InitOptions {
            model_name: EmbeddingModel::AllMiniLML6V2,
            show_download_progress: true,
            ..Default::default()
        })?;
        Ok(Self { model })
    }

    /// Embed a batch of texts. Returns one 384-dim vector per input.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let embeddings = self.model.embed(texts.to_vec(), None)?;
        Ok(embeddings)
    }

    /// Embed a single text.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let mut results = self.embed_batch(&[text])?;
        results.pop().ok_or(EmbeddingError::EmptyResult)
    }

    /// Dimension of the embedding vectors.
    pub fn dimension(&self) -> usize {
        384
    }
}
```

### Tests

- Model loads successfully
- Single text embedding returns 384 dimensions
- Batch embedding returns correct count
- Similar texts produce high cosine similarity
- Dissimilar texts produce low cosine similarity

---

## 9. zen-registry

**Purpose**: HTTP clients for package registries (crates.io, npm, pypi, hex.pm). Pure lookup -- no state mutation.

**Validated in**: Go `internal/registry/registry.go` (working clients for all 4 registries)

### Dependencies

```toml
[dependencies]
zen-core.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

### Module Structure

```
zen-registry/src/
├── lib.rs              # RegistryClient, search_all, PackageInfo
├── crates_io.rs        # crates.io API
├── npm.rs              # npm registry API + api.npmjs.org (downloads)
├── pypi.rs             # PyPI JSON API
└── hex.rs              # hex.pm API
```

### Key Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub ecosystem: String,
    pub description: String,
    pub downloads: u64,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
}

pub struct RegistryClient {
    http: reqwest::Client,
}

impl RegistryClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("zenith/0.1")
                .build()
                .unwrap(),
        }
    }

    /// Search all registries concurrently
    pub async fn search_all(&self, query: &str, limit: usize) -> Vec<PackageInfo> {
        let (crates, npm, pypi, hex) = tokio::join!(
            self.search_crates_io(query, limit),
            self.search_npm(query, limit),
            self.search_pypi(query, limit),
            self.search_hex(query, limit),
        );

        let mut results = Vec::new();
        results.extend(crates.unwrap_or_default());
        results.extend(npm.unwrap_or_default());
        results.extend(pypi.unwrap_or_default());
        results.extend(hex.unwrap_or_default());
        results.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        results
    }

    /// Search a specific ecosystem
    pub async fn search(&self, query: &str, ecosystem: &str, limit: usize) -> Result<Vec<PackageInfo>, RegistryError> {
        match ecosystem {
            "rust" => self.search_crates_io(query, limit).await,
            "npm" => self.search_npm(query, limit).await,
            "pypi" => self.search_pypi(query, limit).await,
            "hex" => self.search_hex(query, limit).await,
            _ => Err(RegistryError::UnsupportedEcosystem(ecosystem.to_string())),
        }
    }
}
```

### Tests

- Each registry client parses real API response format (use recorded JSON fixtures)
- `search_all` merges and sorts results
- Handles API errors gracefully (404, rate limit, timeout)
- npm download count enrichment works

---

## 10. zen-search

**Purpose**: Search orchestration -- ties together zen-db (FTS5), zen-lake (vector), and zen-embeddings. Provides the unified `zen search` command backend.

### Dependencies

```toml
[dependencies]
zen-core.workspace = true
zen-db.workspace = true
zen-lake.workspace = true
zen-embeddings.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
```

### Module Structure

```
zen-search/src/
├── lib.rs              # SearchEngine, SearchResult, SearchMode
├── vector.rs           # Vector search via DuckDB HNSW
├── fts.rs              # FTS5 search via Turso (findings, audit, etc.)
└── hybrid.rs           # Hybrid: vector + FTS combined ranking
```

### Key Types

```rust
pub struct SearchEngine {
    db: ZenDb,
    lake: ZenLake,
    embeddings: EmbeddingEngine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub package: String,
    pub ecosystem: String,
    pub version: String,
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub doc_comment: String,
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub enum SearchMode {
    Vector,
    Fts,
    Hybrid,
}

impl SearchEngine {
    pub async fn search(
        &self,
        query: &str,
        mode: SearchMode,
        filters: SearchFilters,
    ) -> Result<Vec<SearchResult>, SearchError> {
        match mode {
            SearchMode::Vector => self.vector_search(query, &filters).await,
            SearchMode::Fts => self.fts_search(query, &filters).await,
            SearchMode::Hybrid => self.hybrid_search(query, &filters).await,
        }
    }
}
```

### Tests

- Vector search returns results ranked by cosine similarity
- FTS search matches porter-stemmed terms
- Hybrid search combines both scoring methods
- Package and kind filters work correctly
- Context budget truncation works

---

## 11. zen-cli

**Purpose**: CLI binary. Parses commands via clap derive, delegates to the other crates.

**Validated in**: aether `aether-cli` (clap derive pattern)

### Dependencies

```toml
[dependencies]
zen-core.workspace = true
zen-config.workspace = true
zen-db.workspace = true
zen-lake.workspace = true
zen-parser.workspace = true
zen-embeddings.workspace = true
zen-registry.workspace = true
zen-search.workspace = true
clap.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
anyhow.workspace = true
dotenvy.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

[[bin]]
name = "zen"
path = "src/main.rs"
```

### Module Structure

```
zen-cli/src/
├── main.rs             # Entry point, init tracing, load config
├── cli.rs              # Clap derive structs (Cli, Commands, subcommands)
├── commands/
│   ├── mod.rs
│   ├── init.rs         # zen init
│   ├── onboard.rs      # zen onboard
│   ├── session.rs      # zen session {start,end,list}
│   ├── install.rs      # zen install
│   ├── search.rs       # zen search
│   ├── research.rs     # zen research {create,update,list,get,registry}
│   ├── finding.rs      # zen finding {create,update,list,get,tag,untag}
│   ├── hypothesis.rs   # zen hypothesis {create,update,list,get}
│   ├── insight.rs      # zen insight {create,update,list,get}
│   ├── task.rs         # zen task {create,update,list,get,complete}
│   ├── log.rs          # zen log
│   ├── compat.rs       # zen compat {check,list,get}
│   ├── link.rs         # zen link, zen unlink
│   ├── audit.rs        # zen audit
│   ├── whats_next.rs   # zen whats-next
│   └── wrap_up.rs      # zen wrap-up
└── output.rs           # JSON/table/raw formatting
```

### Clap Structure

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "zen", about = "Zenith - developer knowledge toolbox")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format
    #[arg(short, long, default_value = "json")]
    pub format: OutputFormat,

    /// Max results
    #[arg(short, long)]
    pub limit: Option<u32>,

    /// Quiet mode
    #[arg(short, long)]
    pub quiet: bool,

    /// Verbose mode
    #[arg(short, long)]
    pub verbose: bool,

    /// Project root path
    #[arg(short, long)]
    pub project: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize zenith for a project
    Init {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        ecosystem: Option<String>,
        #[arg(long)]
        no_index: bool,
    },
    /// Onboard existing project
    Onboard {
        #[arg(long)]
        workspace: bool,
        #[arg(long)]
        root: Option<String>,
        #[arg(long)]
        skip_indexing: bool,
    },
    /// Session management
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },
    /// Install and index a package
    Install {
        package: String,
        #[arg(long)]
        ecosystem: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        include_tests: bool,
        #[arg(long)]
        force: bool,
    },
    /// Search indexed documentation
    Search {
        query: String,
        #[arg(long)]
        package: Option<String>,
        #[arg(long)]
        ecosystem: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        mode: Option<String>,
        #[arg(long)]
        context_budget: Option<u32>,
    },
    /// Research items
    Research {
        #[command(subcommand)]
        action: ResearchCommands,
    },
    /// Findings
    Finding {
        #[command(subcommand)]
        action: FindingCommands,
    },
    /// Hypotheses
    Hypothesis {
        #[command(subcommand)]
        action: HypothesisCommands,
    },
    /// Insights
    Insight {
        #[command(subcommand)]
        action: InsightCommands,
    },
    /// Tasks
    Task {
        #[command(subcommand)]
        action: TaskCommands,
    },
    /// Log implementation
    Log {
        /// file#start-end format
        location: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Compatibility checks
    Compat {
        #[command(subcommand)]
        action: CompatCommands,
    },
    /// Create entity link
    Link {
        source: String,
        target: String,
        relation: String,
    },
    /// Remove entity link
    Unlink { link_id: String },
    /// View audit trail
    Audit {
        #[arg(long)]
        entity_type: Option<String>,
        #[arg(long)]
        entity_id: Option<String>,
        #[arg(long)]
        action: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        search: Option<String>,
    },
    /// Project state and next steps
    WhatsNext,
    /// End session, sync, summarize
    WrapUp {
        #[arg(long)]
        auto_commit: bool,
        #[arg(long)]
        message: Option<String>,
    },
}

// ... subcommand enums for Session, Research, Finding, etc.
```

### Command Handler Pattern

Each command follows the same pattern:

```rust
// commands/finding.rs
pub async fn handle_finding(
    action: FindingCommands,
    db: &ZenDb,
    format: OutputFormat,
    limit: Option<u32>,
) -> Result<()> {
    match action {
        FindingCommands::Create { content, research, source, confidence, tag } => {
            let finding = Finding {
                id: EntityId::new(PREFIX_FINDING, &content).full,
                research_id: research,
                session_id: db.active_session_id().await?,
                content,
                source,
                confidence: confidence.unwrap_or(Confidence::Medium),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            db.create_finding(&finding).await?;

            for t in tag.unwrap_or_default() {
                db.tag_finding(&finding.id, &t).await?;
            }

            output(&finding, format)?;
        }
        // ... other actions
    }
    Ok(())
}
```

### Tests

- Integration tests: run CLI commands as subprocess, verify JSON output
- Each command produces valid JSON
- Error cases return appropriate error JSON

---

## 12. Implementation Order

### Sprint 1: Foundation

1. **zen-core** -- Types, IDs, errors. No external dependencies beyond serde/chrono/uuid/sha2
2. **zen-config** -- Figment loading. Depends only on zen-core
3. **zen-db** -- Turso schema + CRUD. Depends on zen-core, zen-config

### Sprint 2: Parsing + Indexing

4. **zen-parser** -- ast-grep-based extraction. Depends on zen-core + ast-grep-core + ast-grep-language. Largest crate, most test coverage needed
5. **zen-embeddings** -- fastembed wrapper. Depends only on zen-core
6. **zen-lake** -- DuckDB/DuckLake. Depends on zen-core, zen-config, zen-embeddings

### Sprint 3: Search + Registry

7. **zen-registry** -- HTTP clients. Depends only on zen-core
8. **zen-search** -- Orchestration. Depends on zen-db, zen-lake, zen-embeddings

### Sprint 4: CLI

9. **zen-cli** -- Binary. Depends on everything

### Critical Path

```
zen-core ──► zen-config ──► zen-db ──────────────────────────► zen-cli
    │                                                              ▲
    ├──► zen-parser ─────────────────────────────────────────────┤
    │                                                              │
    ├──► zen-embeddings ──► zen-lake ──► zen-search ─────────────┤
    │                                                              │
    └──► zen-registry ────────────────────────────────────────────┘
```

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- DuckLake data model: [02-ducklake-data-model.md](./02-ducklake-data-model.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- klaw-effect-tracker source: `~/projects/klaw/.agents/skills/klaw-effect-tracker/`
- aether validated patterns: `~/projects/aether/`
