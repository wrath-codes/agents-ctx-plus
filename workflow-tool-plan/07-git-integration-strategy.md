# Git Integration Strategy

## Overview

This document outlines the comprehensive Git integration strategy for the workflow tool, adapting beads' proven patterns to enable robust agent coordination, session management, and conflict-free collaboration using modern Rust crates and best practices.

## Technology Stack

### Core Dependencies

```toml
[dependencies]
# Git operations (pure Rust)
gix = "0.77.0"                    # Pure Rust Git implementation
# JSONL processing (append-only format)
json-lines = "0.1.0"                 # Lightweight, streaming, serde-compatible
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# ID generation and security
sha2 = "0.10"                     # Cryptographic hashes
base32 = "0.4"                     # Base32 encoding
uuid = { version = "1.16.0", features = ["v4", "serde"] }

# File system monitoring
notify = "8.0.0"                  # Cross-platform file watching
async-watcher = "0.4.0"            # Tokio-based file monitoring

# Vector database and search
duckdb = "1.2.0"                    # SQL + vector search (VSS extension)
fastembed = "5.8.1"                 # Local embedding models

# CLI interface and time handling
clap = { version = "4.5.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }

# Async runtime and utilities
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"                       # Error handling
thiserror = "1.0"                     # Custom error types
```

## Architecture

### 3-Layer Git Integration System

```
┌─────────────────────────────────────────────────────────────────┐
│                Git-Integrated Workflow Tool           │
│                                                         │
│  ┌─────────────────┐    ┌─────────────────┐    │
│  │   Agent Layer    │    │  JSONL Storage  │    │
│  │                 │◄──►│                 │◄──►│
│  └─────────────────┘    └─────────────────┘    │
│           │                      │              │
│           ▼                      ▼              │
│  ┌─────────────────────────────────────────┐    │
│  │            Git Repository            │    │
│  │     (Historical Truth)             │    │
│  │                                     │    │
│  │ • Agent operation logs             │    │
│  │ • Complete audit trails           │    │
│  │ • Merge-safe JSONL format       │    │
│  │ • Branch-based isolation         │    │
│  └─────────────────────────────────────────┘    │
│                                                         │
│           ▼                      ▼              │
│  ┌─────────────────────────────────────────┐    │
│  │           Vector Database            │    │
│  │     (Fast Queries + Search)             │    │
│  │ • Agent operation indexing           │    │
│  │ • Semantic search with embeddings   │    │
│  │ • Similarity-based retrieval        │    │
│  └─────────────────────────────────────────┘    │
│                                                         │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Git Repository Manager

**Purpose**: Central Git operations coordination using pure Rust implementation

**Features**:
- Repository creation and branch management
- Agent-specific branch isolation
- Merge-based coordination
- Complete audit trails
- Tag-based session checkpoints

**Implementation**:

```rust
// src/git/repository_manager.rs
use gix::{Repository, ObjectId, Commit, Reference};
use anyhow::Result;
use std::path::Path;

pub struct GitRepositoryManager {
    repo: Repository,
    branch_name: String,
}

impl GitRepositoryManager {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo = Repository::open(repo_path)?;
        let branch_name = repo.head()?.shorthand()
            .unwrap_or("main")
            .to_string();
            
        Ok(Self { repo, branch_name })
    }
    
    pub fn create_agent_branch(&self, agent_type: &AgentType) -> Result<String> {
        let branch_name = format!("agent/{}/{}-{}", 
            self.session_id, 
            agent_type.to_string()
        );
        
        // Create branch from current HEAD
        let head_commit = self.repo.head()?.peel_to_commit()?;
        self.repo.branch(
            &branch_name,
            &head_commit,
            false, // force
        )?;
        
        Ok(branch_name)
    }
    
    pub fn commit_agent_operation(&self, operation: &AgentOperation) -> Result<ObjectId> {
        // Append to JSONL
        let jsonl_path = self.get_agent_jsonl_path(&operation.agent_type);
        self.append_to_jsonl(&jsonl_path, operation)?;
        
        // Stage and commit
        let mut index = self.repo.index()?;
        index.add_path(Path::new(&jsonl_path))?;
        let tree_id = index.write_tree()?;
        
        let tree = self.repo.find_tree(tree_id)?;
        let parent = self.repo.head()?.peel_to_commit()?;
        let signature = self.repo.signature()?;
        
        let commit_id = self.repo.commit(
            &format!("refs/heads/{}", self.branch_name),
            &signature,
            &signature,
            &operation.generate_commit_message(),
            &tree,
            &[&parent],
        )?;
        
        Ok(commit_id)
    }
    
    pub async fn merge_agent_branches(&self, agent_branches: &[String]) -> Result<Vec<AgentOperation>> {
        let mut all_operations = Vec::new();
        
        // Switch to main
        self.repo.set_head("refs/heads/main")?;
        
        for branch_name in agent_branches {
            // Merge agent branch
            let branch_ref = self.repo.find_reference(&format!("refs/heads/{}", branch_name))?;
            let branch_commit = branch_ref.peel_to_commit()?;
            
            // Analyze merge
            let merge_result = self.repo.merge_analysis(&branch_commit, Some(&self.get_head_commit()?))?;
            
            if merge_result.is_fast_forward() {
                // Fast-forward merge
                self.repo.set_head(&format!("refs/heads/{}", branch_name))?;
            } else {
                // Need merge commit
                let operations = self.extract_operations_from_merge(&branch_commit)?;
                all_operations.extend(operations);
                
                // Create merge commit
                self.create_merge_commit(branch_name, &branch_commit)?;
            }
        }
        
        Ok(all_operations)
    }
}
```

### 2. Append-Only JSONL Storage

**Purpose**: Beads-style merge-conflict prevention with streaming support

**Features**:
- Append-only format (never modifies existing lines)
- Serde-compatible serialization
- Streaming support for large files
- Agent-specific JSONL files

**Implementation**:

```rust
// src/storage/jsonl_store.rs
use json_lines::{JsonLinesReader, JsonLinesWriter};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct AppendOnlyJsonlStore<T> 
where 
    T: for<'de> Deserialize<'de> + Serialize,
{
    file_path: PathBuf,
}

impl<T> AppendOnlyJsonlStore<T>
where 
    T: for<'de> Deserialize<'de> + Serialize,
{
    pub fn new(file_path: &Path) -> Self {
        Self {
            file_path: file_path.to_path_buf(),
        }
    }
    
    pub fn append(&self, item: &T) -> Result<()> {
        let mut writer = JsonLinesWriter::new(&self.file_path)?;
        writer.write(item)?;
        Ok(())
    }
    
    pub fn read_recent(&self, limit: Option<usize>) -> Result<Vec<T>> {
        if !self.file_path.exists() {
            return Ok(vec![]);
        }
        
        let reader = JsonLinesReader::new(&self.file_path)?;
        let mut items = Vec::new();
        
        for (i, result) in reader.into_iter().enumerate() {
            if let Some(item) = result {
                items.push(item);
                
                if let Some(limit) = limit {
                    if i >= limit - 1 {
                        break;
                    }
                }
            }
        }
        
        // Return in reverse order (newest first)
        items.reverse();
        Ok(items)
    }
    
    pub fn read_streaming<'a>(&'a self) -> Result<impl Iterator<Item = Result<T>> + 'a> {
        if !self.file_path.exists() {
            return Ok(Box::new(std::iter::empty()));
        }
        
        let reader = JsonLinesReader::new(&self.file_path)?;
        Ok(Box::new(reader.into_iter()))
    }
}

// Agent-specific storage types
pub type AgentOperationStore = AppendOnlyJsonlStore<AgentOperation>;
pub type ResearchAgentStore = AppendOnlyJsonlStore<ResearchOperation>;
pub type POCAgentStore = AppendOnlyJsonlStore<POCOperation>;
pub type DocumentationAgentStore = AppendOnlyJsonlStore<DocumentationOperation>;
pub type ValidationAgentStore = AppendOnlyJsonlStore<ValidationOperation>;
```
- Automatic error recovery

### 3. Agent Branch Coordinator

**Purpose**: Multi-agent workflow coordination with Git-based isolation

**Features**:
- Dedicated branches for each agent type
- Dependency-aware workflow execution
- Cross-agent conflict prevention
- Automatic branch creation and cleanup

**Implementation**:

```rust
// src/agents/branch_coordinator.rs
use crate::git::repository_manager::GitRepositoryManager;
use crate::storage::jsonl_store::AgentOperationStore;
use anyhow::Result;
use tokio::sync::mpsc;

pub struct AgentBranchCoordinator {
    git_manager: GitRepositoryManager,
    operation_store: AgentOperationStore,
    session_id: String,
}

impl AgentBranchCoordinator {
    pub fn new(git_manager: GitRepositoryManager, session_id: String) -> Self {
        Self {
            git_manager,
            operation_store: AgentOperationStore::new(
                &git_manager.repo.workdir().unwrap()
                    .join(".workflow-tool/agents")
            ),
            session_id,
        }
    }
    
    pub async fn setup_agent_branches(&self) -> Result<Vec<String>> {
        let agent_types = [
            AgentType::Research,
            AgentType::POC,
            AgentType::Documentation,
            AgentType::Validation,
            AgentType::Supervisor,
        ];
        
        let mut agent_branches = Vec::new();
        
        for agent_type in agent_types {
            let branch_name = self.git_manager.create_agent_branch(&agent_type)?;
            agent_branches.push(branch_name);
            
            println!("🌿 Created agent branch: {}", branch_name);
        }
        
        Ok(agent_branches)
    }
    
    pub async fn coordinate_agent_workflow(&self) -> Result<Vec<AgentOperation>> {
        let (tx, mut rx) = mpsc::channel::<AgentProgress>(100);
        
        // Start agent monitoring
        let monitor_task = tokio::spawn({
            let session_id = self.session_id.clone();
            async move {
                Self::monitor_agent_progress(session_id, tx).await;
            }
        });
        
        // Execute workflow phases
        let operations = self.execute_workflow_phases().await?;
        
        // Wait for all agents to complete
        let _ = monitor_task.await;
        
        Ok(operations)
    }
    
    async fn execute_workflow_phases(&self) -> Result<Vec<AgentOperation>> {
        let mut all_operations = Vec::new();
        
        // Phase 1: Research
        all_operations.extend(self.execute_research_phase().await?);
        
        // Phase 2: POC Implementation
        all_operations.extend(self.execute_poc_phase().await?);
        
        // Phase 3: Validation
        all_operations.extend(self.execute_validation_phase().await?);
        
        // Phase 4: Documentation
        all_operations.extend(self.execute_documentation_phase().await?);
        
        // Phase 5: Supervisor Coordination
        all_operations.extend(self.execute_supervisor_phase().await?);
        
        Ok(all_operations)
    }
    
    async fn execute_research_phase(&self) -> Result<Vec<AgentOperation>> {
        println!("🔬 Starting Research Phase...");
        
        // Switch to research agent branch
        self.git_manager.switch_to_branch("agent/research-research").await?;
        
        // Execute research agent (this would call the actual ResearchAgent)
        let operations = self.mock_research_operations().await?;
        
        println!("🔬 Research Phase completed with {} operations", operations.len());
        Ok(operations)
    }
    
    async fn execute_poc_phase(&self) -> Result<Vec<AgentOperation>> {
        println!("⚡ Starting POC Phase...");
        
        // Switch to POC agent branch
        self.git_manager.switch_to_branch("agent/poc-poc").await?;
        
        // Execute POC agent with research findings
        let research_findings = self.get_research_findings().await?;
        let operations = self.mock_poc_operations(&research_findings).await?;
        
        println!("⚡ POC Phase completed with {} operations", operations.len());
        Ok(operations)
    }
    
    async fn execute_supervisor_phase(&self) -> Result<Vec<AgentOperation>> {
        println!("🎯 Starting Supervisor Coordination Phase...");
        
        // Switch to main branch
        self.git_manager.switch_to_branch("main").await?;
        
        // Merge all agent branches
        let agent_branches = vec![
            "agent/research-research",
            "agent/poc-poc", 
            "agent/documentation-documentation",
            "agent/validation-validation",
        ];
        
        let operations = self.git_manager.merge_agent_branches(&agent_branches).await?;
        
        println!("🎯 Supervisor Coordination completed with {} total operations", operations.len());
        Ok(operations)
    }
}
```

### 4. Collision-Resistant ID System

**Purpose**: Unique agent operation identification following beads patterns

**Features**:
- SHA256 + base32 encoding
- Agent-type prefixes (res, poca, doc, val, sup)
- Timestamp + UUID collision resistance
- Git-friendly commit message generation

**Implementation**:

```rust
// src/core/agent_id.rs
use uuid::Uuid;
use sha2::{Sha256, Digest};
use base32::Alphabet;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    #[serde(rename = "research")]
    Research,
    #[serde(rename = "poc")]
    POC,
    #[serde(rename = "documentation")]
    Documentation,
    #[serde(rename = "validation")]
    Validation,
    #[serde(rename = "supervisor")]
    Supervisor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOperationId {
    pub agent_type: AgentType,
    pub short_id: String,
    pub full_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl AgentOperationId {
    pub fn new(agent_type: AgentType, title: &str) -> Self {
        let timestamp = Utc::now();
        let uuid_val = Uuid::new_v4();
        
        // Create collision-resistant hash
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}:{}:{}", 
            agent_type.to_string().to_lowercase(),
            title.to_lowercase().trim(),
            timestamp.to_rfc3339(),
            uuid_val.to_string()
        ).as_bytes());
        
        let hash_result = hasher.finalize();
        let encoded = base32::encode(Alphabet::RFC4648 { padding: true }, &hash_result);
        let short_id = encoded[..8].to_lowercase();
        
        let agent_prefix = match agent_type {
            AgentType::Research => "res",
            AgentType::POC => "poca",
            AgentType::Documentation => "doc",
            AgentType::Validation => "val",
            AgentType::Supervisor => "sup",
        };
        
        let full_id = format!("{}-{}", agent_prefix, short_id);
        
        Self {
            agent_type,
            short_id,
            full_id,
            timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOperation {
    pub id: AgentOperationId,
    pub operation_type: OperationType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub title: String,
    pub data: serde_json::Value,
    pub dependencies: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl AgentOperation {
    pub fn generate_commit_message(&self) -> String {
        match self.operation_type {
            OperationType::ResearchFinding => 
                format!("ResearchAgent: Found {}", self.title),
            OperationType::POCImplementation => 
                format!("POCAgent: Implemented {}", self.title),
            OperationType::DocumentationGeneration => 
                format!("DocumentationAgent: Generated {}", self.title),
            OperationType::ValidationResult => 
                format!("ValidationAgent: Validated {}", self.title),
            OperationType::Coordination => 
                format!("SupervisorAgent: Coordinated {}", self.title),
        }
    }
}
```

### 5. Vector Database Integration

**Purpose**: Semantic search across agent operations using DuckDB + VSS

**Features**:
- Local embedding generation with fastembed
- Vector similarity search with HNSW indexing
- Hybrid text + vector queries
- Performance optimization for large datasets

**Implementation**:

```rust
// src/storage/vector_database.rs
use duckdb::{Connection, Result as DuckResult};
use anyhow::Result;
use serde_json::Value;
use fastembed::{TextEmbedding, EmbeddingModel};

pub struct VectorDatabase {
    conn: Connection,
    embedding_model: TextEmbedding,
}

impl VectorDatabase {
    pub async fn new() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        
        // Enable VSS extension
        conn.execute_batch(&["INSTALL vss;", "LOAD vss;"])?;
        
        // Initialize embedding model
        let model = TextEmbedding::try_new(Default::default())?;
        
        Ok(Self {
            conn,
            embedding_model: model,
        })
    }
    
    pub fn setup_vector_tables(&self) -> Result<()> {
        // Create tables for agent operations with vector search
        self.conn.execute("
            CREATE TABLE IF NOT EXISTS agent_operations (
                id TEXT PRIMARY KEY,
                agent_type TEXT NOT NULL,
                operation_type TEXT NOT NULL,
                timestamp TIMESTAMP NOT NULL,
                title TEXT NOT NULL,
                data JSON NOT NULL,
                dependencies TEXT,
                metadata JSON,
                embedding FLOAT[384],
                vector_index HNSW(embedding)
            )
        ")?;
        
        Ok(())
    }
    
    pub async fn store_operation(&mut self, operation: &AgentOperation) -> Result<()> {
        // Generate embedding for operation title and data
        let text_to_embed = format!("{} {}", operation.title, operation.data);
        let embedding = self.embedding_model.embed(text_to_embed, None)?;
        
        // Store with vector
        let dependencies_json = serde_json::to_string(&operation.dependencies)?;
        let metadata_json = serde_json::to_string(&operation.metadata)?;
        
        self.conn.execute("
            INSERT INTO agent_operations 
            (id, agent_type, operation_type, timestamp, title, data, dependencies, metadata, embedding)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ", 
        (&operation.id.full_id, 
         operation.id.agent_type.to_string(), 
         operation.operation_type.to_string(),
         operation.timestamp,
         &operation.title,
         &serde_json::to_string(&operation.data),
         &dependencies_json,
         &metadata_json,
         &embedding.vec
        ))?;
        
        Ok(())
    }
    
    pub async fn search_similar_operations(&self, query: &str, agent_types: &[AgentType]) -> Result<Vec<AgentOperation>> {
        // Generate query embedding
        let query_embedding = self.embedding_model.embed(query, None)?;
        
        let agent_types_str = agent_types.iter()
            .map(|t| format!("'{}'", t.to_string()))
            .collect::<Vec<_>>()
            .join(",");
        
        // Execute vector similarity search
        let mut stmt = self.conn.prepare("
            SELECT id, agent_type, operation_type, timestamp, title, data, dependencies, metadata
            FROM agent_operations
            WHERE agent_type IN ([])
            ORDER BY array_cosine_similarity(embedding, ?) DESC
            LIMIT 10
        ", agent_types_str)?;
        
        stmt.bind(1, &query_embedding.vec)?;
        
        let mut results = Vec::new();
        while let Some(row) = stmt.next()? {
            results.push(AgentOperation {
                id: AgentOperationId::from_string(row.get(0)?),
                agent_type: AgentType::from_string(row.get(1)?),
                operation_type: OperationType::from_string(row.get(2)?),
                timestamp: chrono::DateTime::parse_from_rfc3339(row.get(3)?)?,
                title: row.get(4)?,
                data: serde_json::from_str(row.get(5)?)?,
                dependencies: serde_json::from_str(row.get(6)?)?,
                metadata: serde_json::from_str(row.get(7)?)?,
            });
        }
        
        Ok(results)
    }
    
    pub fn rebuild_from_jsonl(&self, jsonl_dir: &Path) -> Result<()> {
        let operations = self.read_all_jsonl_files(jsonl_dir)?;
        
        // Batch insert operations
        self.conn.execute("BEGIN TRANSACTION")?;
        
        for operation in operations {
            self.store_operation(&operation).await?;
        }
        
        self.conn.execute("COMMIT")?;
        
        // Create vector index
        self.conn.execute("CREATE INDEX IF NOT EXISTS idx_agent_operations_embedding ON agent_operations USING HNSW(embedding)")?;
        
        Ok(())
    }
}
```

### 6. Git Hooks System

**Purpose**: Automation and validation integrated with Git workflow

**Features**:
- Pre-commit JSONL validation
- Post-checkout context rebuilding
- Post-merge coordination triggers
- Agent-aware conflict detection
- Automatic error recovery

**Implementation**:

```rust
// src/git/hooks_manager.rs
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct GitHooksManager {
    repo_path: PathBuf,
    hooks_dir: PathBuf,
}

impl GitHooksManager {
    pub fn new(repo_path: &Path) -> Self {
        let hooks_dir = repo_path.join(".workflow-tool/hooks");
        Self {
            repo_path: repo_path.to_path_buf(),
            hooks_dir,
        }
    }
    
    pub async fn install_hooks(&self) -> Result<()> {
        // Create hooks directory
        fs::create_dir_all(&self.hooks_dir).await?;
        
        // Install hooks
        self.create_pre_commit_hook().await?;
        self.create_post_checkout_hook().await?;
        self.create_post_merge_hook().await?;
        self.copy_to_git_hooks().await?;
        
        Ok(())
    }
    
    async fn create_pre_commit_hook(&self) -> Result<()> {
        let hook_content = r#"#!/bin/bash
# Validate JSONL format before commit
echo "🔍 Validating agent JSONL files..."

# Find all agent JSONL files
JSONL_FILES=$(find .workflow-tool/agents -name "*.jsonl" 2>/dev/null || true)

for file in $JSONL_FILES; do
    if [ -f "$file" ]; then
        # Validate JSONL format using jq if available
        if command -v jq >/dev/null 2>&1; then
            while IFS= read -r line; do
                if [ -n "$line" ]; then
                    if ! echo "$line" | jq empty 2>/dev/null; then
                        echo "❌ Invalid JSON in $file: $line"
                        echo "Please fix JSONL format before committing."
                        exit 1
                    fi
                fi
            done < "$file"
        else
            # Fallback validation using python
            python3 -c "
import json, sys
try:
    with open('$file', 'r') as f:
        for i, line in enumerate(f):
            if line.strip() and not line.strip().startswith('#'):
                json.loads(line)
except json.JSONDecodeError as e:
    print(f'❌ Invalid JSON in {file}: {e}')
    sys.exit(1)
            "
        fi
    fi
done

echo "✅ JSONL validation passed"
"#;
        
        let hook_path = self.hooks_dir.join("pre-commit");
        fs::write(&hook_path, hook_content).await?;
        
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms).await?;
        }
        
        Ok(())
    }
    
    async fn create_post_checkout_hook(&self) -> Result<()> {
        let hook_content = r#"#!/bin/bash
# Rebuild agent context when switching branches
BRANCH_NAME=$(git rev-parse --abbrev-ref HEAD)
echo "🔄 Switched to branch: $BRANCH_NAME"

# Rebuild agent context for new branch
if command -v workflow-tool >/dev/null 2>&1; then
    workflow-tool rebuild-context --branch "$BRANCH_NAME"
    echo "🔄 Agent context rebuilt for branch: $BRANCH_NAME"
else
    echo "⚠️  workflow-tool not found in PATH - manual context rebuild required"
fi
"#;
        
        let hook_path = self.hooks_dir.join("post-checkout");
        fs::write(&hook_path, hook_content).await?;
        self.make_executable(&hook_path).await?;
        Ok(())
    }
    
    async fn create_post_merge_hook(&self) -> Result<()> {
        let hook_content = r#"#!/bin/bash
# Handle cross-agent coordination after merge
echo "🤝 Processing merge completion..."

# Check for merge conflicts in agent files
JSONL_FILES=$(find .workflow-tool -name "*.jsonl" 2>/dev/null || true)

for file in $JSONL_FILES; do
    if [ -f "$file" ]; then
        # Check for Git conflict markers
        if grep -q "^<<<<<<<\|^=======\|^>>>>>>>" "$file"; then
            echo "❌ Merge conflicts detected in $file"
            echo "Please resolve conflicts before continuing."
            exit 1
        fi
    fi
done

# Rebuild agent coordination
if command -v workflow-tool >/dev/null 2>&1; then
    workflow-tool coordinate-agents --from-merge
    echo "🤝 Agent coordination updated after merge"
fi
"#;
        
        let hook_path = self.hooks_dir.join("post-merge");
        fs::write(&hook_path, hook_content).await?;
        self.make_executable(&hook_path).await?;
        Ok(())
    }
    
    async fn copy_to_git_hooks(&self) -> Result<()> {
        let git_hooks_dir = self.repo_path.join(".git").join("hooks");
        fs::create_dir_all(&git_hooks_dir).await?;
        
        let hooks = ["pre-commit", "post-checkout", "post-merge"];
        for hook in hooks {
            let src = self.hooks_dir.join(hook);
            let dst = git_hooks_dir.join(hook);
            
            if src.exists() {
                fs::copy(&src, &dst).await?;
                self.make_executable(&dst).await?;
            }
        }
        
        Ok(())
    }
    
    async fn make_executable(&self, path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms).await?;
        }
        Ok(())
    }
}
```

### 7. Session Management

**Purpose**: Git-based session persistence and restoration

**Features**:
- Branch-based session isolation
- JSONL state persistence
- Tag-based checkpointing
- Cross-session agent handoffs
- Complete session audit trails

**Implementation**:

```rust
// src/session/session_manager.rs
use crate::git::repository_manager::GitRepositoryManager;
use crate::storage::jsonl_store::AppendOnlyJsonlStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBackedSession {
    pub id: String,
    pub branch_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub agent_states: std::collections::HashMap<String, serde_json::Value>,
    pub context_builds: Vec<ContextBuild>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl GitBackedSession {
    pub async fn create(git_manager: &GitRepositoryManager, session_id: &str) -> Result<Self> {
        // Create session branch
        let branch_name = format!("session/{}", session_id);
        
        // Create session struct
        let session = Self {
            id: session_id.to_string(),
            branch_name: branch_name.clone(),
            created_at: Utc::now(),
            agent_states: std::collections::HashMap::new(),
            context_builds: Vec::new(),
            metadata: std::collections::HashMap::new(),
        };
        
        // Save session to Git
        session.save_session_state(git_manager).await?;
        
        Ok(session)
    }
    
    pub async fn save_session_state(&self, git_manager: &GitRepositoryManager) -> Result<()> {
        let session_json = serde_json::to_string_pretty(self)?;
        let session_path = git_manager.repo.workdir().unwrap().join(".workflow-tool").join("sessions");
        std::fs::create_dir_all(&session_path)?;
        
        let session_file = session_path.join(format!("{}.json", self.id));
        std::fs::write(session_file, session_json)?;
        
        // Append to sessions.jsonl
        let jsonl_path = session_path.join("sessions.jsonl");
        let jsonl_line = serde_json::to_string(self)?;
        
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_path)?;
            
        writeln!(file, "{}", jsonl_line)?;
        
        Ok(())
    }
    
    pub async fn restore(git_manager: &GitRepositoryManager, session_id: &str) -> Result<Self> {
        // Find session in sessions.jsonl
        let sessions_jsonl = git_manager.repo.workdir().unwrap().join(".workflow-tool/sessions/sessions.jsonl");
        
        if !sessions_jsonl.exists() {
            return Err(anyhow::anyhow!("No sessions found"));
        }
        
        let content = std::fs::read_to_string(&sessions_jsonl)?;
        
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            let session: GitBackedSession = serde_json::from_str(line)?;
            if session.id == session_id {
                // Checkout session branch
                let repo = Repository::open(git_manager.repo.workdir().unwrap())?;
                
                // Check if session branch exists
                match repo.find_branch(&session.branch_name, BranchType::Local) {
                    Ok(branch) => {
                        // Checkout existing branch
                        repo.set_head(branch.get().name().unwrap())?;
                        let tree = branch.get().peel_to_tree()?;
                        repo.checkout_tree(tree.as_tree(), None)?;
                    },
                    Err(_) => {
                        // Create session branch from session tag
                        let tag_name = format!("session/{}", session_id);
                        if let Ok(tag) = repo.find_tag(&tag_name) {
                            let commit = tag.peel_to_commit()?;
                            repo.branch(&session.branch_name, &commit, false)?;
                            repo.set_head(&format!("refs/heads/{}", session.branch_name))?;
                        }
                    }
                }
                
                return Ok(session);
            }
        }
        
        Err(anyhow::anyhow!("Session {} not found", session_id))
    }
    
    pub async fn checkpoint_session(&self, git_manager: &GitRepositoryManager) -> Result<Oid> {
        // Save session state
        self.save_session_state(git_manager).await?;
        
        // Stage changes
        let repo = Repository::open(git_manager.repo.workdir().unwrap())?;
        let mut index = repo.index()?;
        
        // Add session files
        index.add_path(&PathBuf::from(".workflow-tool/sessions"))?;
        let tree_id = index.write_tree()?;
        
        // Create checkpoint commit
        let tree = repo.find_tree(tree_id)?;
        let parent = repo.head()?.peel_to_commit()?;
        let signature = repo.signature()?;
        
        let commit_id = repo.commit(
            Some(&format!("refs/heads/{}", self.branch_name)),
            &signature,
            &signature,
            &format!("Session checkpoint: {}", self.id),
            &tree,
            &[&parent],
        )?;
        
        // Create tag for easy retrieval
        repo.tag(&format!("session-checkpoint/{}", self.id), 
                 &repo.find_object(commit_id, None)?, 
                 &signature, 
                 &format!("Session checkpoint: {}", self.id), 
                 false)?;
        
        Ok(commit_id)
    }
}
```

## Implementation Patterns

### Git-Based Agent Isolation

```bash
# Create agent-specific branches
git checkout -b agent/research-session-001
git checkout -b agent/poc-session-001  
git checkout -b agent/documentation-session-001
git checkout -b agent/validation-session-001

# Each agent works in isolation
workflow-tool research --session session-001 --branch agent/research
workflow-tool poc --session session-001 --branch agent/poc

# Merge completed work
git checkout main
git merge agent/research-session-001  # Research findings
git merge agent/poc-session-001     # POC implementations
git merge agent/documentation-session-001  # Documentation
git merge agent/validation-session-001  # Validation results
```

### Append-Only JSONL Format

```jsonl
// research-agent.jsonl (append-only)
{"id": {"agent_type": "research", "short_id": "a1b2", "full_id": "res-a1b2"}, "operation_type": "finding", "timestamp": "2026-02-06T10:00:00Z", "title": "Fastembed models comparison", "data": {...}, "dependencies": [], "metadata": {...}}

{"id": {"agent_type": "poc", "short_id": "c3d4", "full_id": "poca-c3d4"}, "operation_type": "implementation", "timestamp": "2026-02-06T11:00:00Z", "title": "Fastembed integration test", "data": {...}, "dependencies": ["res-a1b2"], "metadata": {...}}

## Additional Components

### File System Monitoring

**Purpose**: Real-time file change detection for agent coordination

**Implementation**:

```rust
// src/monitoring/file_watcher.rs
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use anyhow::Result;
use tokio::sync::mpsc;
use std::path::Path;

pub struct AgentFileWatcher {
    watcher: RecommendedWatcher,
    event_tx: mpsc::UnboundedSender<FileChangeEvent>,
}

#[derive(Debug, Clone)]
pub enum FileChangeEvent {
    AgentJsonlUpdated { agent_type: AgentType, operation_id: String },
    GitStateChanged { new_branch: String },
    ConflictDetected { file_path: String },
    SessionRestored { session_id: String },
}

impl AgentFileWatcher {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                match res {
                    Ok(event) => {
                        let _ = tx.blocking_send(Self::handle_file_event(event));
                    },
                    Err(e) => {
                        eprintln!("File watcher error: {:?}", e);
                    }
                }
            }
        )?;
        
        Ok(Self { watcher, event_tx })
    }
    
    pub fn start_watching(&mut self, watch_path: &Path) -> Result<()> {
        self.watcher.watch(watch_path, RecursiveMode::Recursive)?;
        Ok(())
    }
    
    pub async fn subscribe_to_changes(&mut self) -> mpsc::UnboundedReceiver<FileChangeEvent> {
        // Return a cloned receiver for async consumption
        self.event_tx.clone().subscribe()
    }
    
    fn handle_file_event(event: Event) -> Option<FileChangeEvent> {
        match event.kind {
            EventKind::Create(_) => {
                if let Some(path) = event.paths.first() {
                    if let Some(agent_type) = Self::extract_agent_type(path) {
                        return Some(FileChangeEvent::AgentJsonlUpdated {
                            agent_type,
                            operation_id: "unknown".to_string(),
                        });
                    }
                }
            },
            EventKind::Modify(_) => {
                if let Some(path) = event.paths.first() {
                    Self::handle_jsonl_modification(path)
                }
            },
            _ => None,
        }
    }
    
    fn extract_agent_type(path: &Path) -> Option<AgentType> {
        if let Some(file_name) = path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                if name_str.starts_with("research-") {
                    return Some(AgentType::Research);
                } else if name_str.starts_with("poc-") {
                    return Some(AgentType::POC);
                } else if name_str.starts_with("documentation-") {
                    return Some(AgentType::Documentation);
                } else if name_str.starts_with("validation-") {
                    return Some(AgentType::Validation);
                } else if name_str.starts_with("supervisor-") {
                    return Some(AgentType::Supervisor);
                }
            }
        }
        None
    }
}
```

### Cross-Agent Dependency Tracking

**Purpose**: Track and resolve dependencies between agent operations via Git

**Implementation**:

```rust
// src/dependencies/dependency_manager.rs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentDependency {
    pub from_operation_id: String,
    pub to_operation_id: String,
    pub dependency_type: DependencyType,
    pub status: DependencyStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DependencyType {
    ResearchToPOC,      // POC depends on research finding
    POCToValidation,    // Validation depends on POC result
    ValidationToDocumentation, // Documentation depends on validation
    Sequential,          // Sequential dependency
    Parallel,           // Parallel workflow
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DependencyStatus {
    Pending,
    Ready,
    Blocked,
    Satisfied,
}

pub struct GitDependencyTracker {
    repo: Repository,
    dependencies: Vec<AgentDependency>,
}

impl GitDependencyTracker {
    pub fn track_agent_dependencies(&mut self, operations: &[AgentOperation]) -> Result<()> {
        // Analyze operations and extract dependencies
        let mut new_dependencies = Vec::new();
        
        for operation in operations {
            let deps = self.extract_dependencies_from_operation(operation)?;
            new_dependencies.extend(deps);
        }
        
        // Merge with existing dependencies
        self.dependencies.extend(new_dependencies);
        
        // Save dependencies to Git
        self.save_dependencies().await?;
        
        Ok(())
    }
    
    fn extract_dependencies_from_operation(&self, operation: &AgentOperation) -> Result<Vec<AgentDependency>> {
        let mut dependencies = Vec::new();
        
        // Extract dependencies from operation data
        if let Some(deps) = operation.data.get("dependencies") {
            if let Some(dep_array) = deps.as_array() {
                for dep in dep_array {
                    if let Some(dep_id) = dep.as_str() {
                        let dependency = AgentDependency {
                            from_operation_id: dep_id.to_string(),
                            to_operation_id: operation.id.full_id.clone(),
                            dependency_type: self.infer_dependency_type(operation)?,
                            status: DependencyStatus::Pending,
                        };
                        dependencies.push(dependency);
                    }
                }
            }
        }
        
        Ok(dependencies)
    }
    
    pub fn get_ready_operations(&self) -> Vec<String> {
        self.dependencies
            .iter()
            .filter(|dep| dep.status == DependencyStatus::Ready)
            .map(|dep| dep.to_operation_id.clone())
            .collect()
    }
    
    pub fn update_dependency_status(&mut self, operation_id: &str, status: DependencyStatus) {
        for dependency in &mut self.dependencies {
            if dependency.from_operation_id == operation_id || 
               dependency.to_operation_id == operation_id {
                dependency.status = status.clone();
            }
        }
    }
    
    async fn save_dependencies(&self) -> Result<()> {
        let deps_json = serde_json::to_string_pretty(&self.dependencies)?;
        let deps_path = self.repo.workdir().unwrap().join(".workflow-tool/dependencies.json");
        std::fs::write(deps_path, deps_json)?;
        
        // Commit dependency changes
        let mut index = self.repo.index()?;
        index.add_path(Path::new(".workflow-tool/dependencies.json"))?;
        let tree_id = index.write_tree()?;
        
        let tree = self.repo.find_tree(tree_id)?;
        let parent = match self.repo.head() {
            Ok(head_ref) => Some(head_ref.peel_to_commit()?),
            Err(_) => None,
        };
        
        let signature = self.repo.signature()?;
        
        let commit_id = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Update agent dependencies",
            &tree,
            parent.as_ref().map(|p| &**p).into_iter().collect::<Vec<_>>().as_slice(),
        )?;
        
        Ok(commit_id)
    }
    
    pub async fn resolve_dependency_graph(&self) -> Result<Vec<Vec<String>>> {
        // Build dependency graph and resolve execution order
        use std::collections::{HashMap, HashSet};
        
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut nodes: HashSet<String> = HashSet::new();
        
        // Build graph
        for dependency in &self.dependencies {
            if dependency.status == DependencyStatus::Satisfied {
                nodes.insert(dependency.from_operation_id.clone());
                nodes.insert(dependency.to_operation_id.clone());
                
                graph.entry(dependency.from_operation_id.clone())
                      .or_insert_with(Vec::new)
                      .push(dependency.to_operation_id.clone());
                
                *in_degree.entry(dependency.to_operation_id.clone()).or_insert(0) += 1;
                in_degree.entry(dependency.from_operation_id.clone()).or_insert(0);
            }
        }
        
        // Topological sort
        let mut result = Vec::new();
        let mut queue: Vec<String> = nodes.iter()
            .filter(|node| in_degree.get(*node).unwrap_or(&0) == &0)
            .map(|s| s.clone())
            .collect();
        
        while let Some(node) = queue.pop() {
            result.push(node.clone());
            
            if let Some(dependents) = graph.get(&node) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(dependent.clone());
                        }
                    }
                }
            }
        }
        
        Ok(vec![result])
    }
}
```

### Offline-First Operations

**Purpose**: Enable agents to work without network dependency

**Implementation**:

```rust
// src/git/offline_mode.rs
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct OfflineModeManager {
    repo: Repository,
    is_online: bool,
    operation_queue: VecDeque<AgentOperation>,
    sync_channel: broadcast::Sender<SyncEvent>,
}

#[derive(Debug, Clone)]
pub enum SyncEvent {
    Online,
    Offline,
    SyncStart,
    SyncComplete,
    SyncError(String),
}

impl OfflineModeManager {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let (sync_tx, _) = broadcast::channel(100);
        
        Ok(Self {
            repo: Repository::open(repo_path)?,
            is_online: true,
            operation_queue: VecDeque::new(),
            sync_channel: sync_tx,
        })
    }
    
    pub async fn start_network_monitoring(&mut self) -> Result<()> {
        let mut sync_tx = self.sync_channel.clone();
        
        // Monitor network connectivity
        tokio::spawn(async move {
            let mut last_status = true;
            
            loop {
                let current_status = self.check_connectivity().await;
                
                if current_status != last_status {
                    let event = if current_status { SyncEvent::Online } else { SyncEvent::Offline };
                    let _ = sync_tx.send(event);
                    last_status = current_status;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        });
        
        // Handle sync events
        let mut rx = self.sync_channel.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                match event {
                    SyncEvent::Online => {
                        println!("🌐 Network available - syncing queued operations");
                        // Sync queued operations
                    },
                    SyncEvent::Offline => {
                        println!("📴 Network unavailable - entering offline mode");
                    },
                    SyncEvent::SyncStart => {
                        println!("🔄 Starting sync...");
                    },
                    SyncEvent::SyncComplete => {
                        println!("✅ Sync completed");
                    },
                    SyncEvent::SyncError(err) => {
                        eprintln!("❌ Sync error: {}", err);
                    },
                }
            }
        });
        
        Ok(())
    }
    
    pub async fn execute_agent_operation(&mut self, operation: AgentOperation) -> Result<()> {
        if self.is_online {
            // Try online execution
            match self.execute_online(&operation).await {
                Ok(_) => {
                    // Operation succeeded online, commit immediately
                    self.commit_operation_to_git(&operation)?;
                    Ok(())
                },
                Err(e) => {
                    // Online failed, queue for later
                    println!("⚠️  Online operation failed: {}. Queueing for later sync.", e);
                    self.operation_queue.push_back(operation);
                    Ok(())
                }
            }
        } else {
            // Offline mode - queue operation
            println!("📴 Offline mode - queuing operation: {}", operation.title);
            self.operation_queue.push_back(operation);
            Ok(())
        }
    }
    
    async fn sync_queued_operations(&mut self) -> Result<()> {
        if self.operation_queue.is_empty() {
            return Ok(());
        }
        
        let _ = self.sync_channel.send(SyncEvent::SyncStart);
        
        let mut synced_operations = Vec::new();
        
        while let Some(operation) = self.operation_queue.pop_front() {
            match self.execute_online(&operation).await {
                Ok(_) => {
                    self.commit_operation_to_git(&operation)?;
                    synced_operations.push(operation);
                },
                Err(e) => {
                    eprintln!("❌ Failed to sync operation {}: {}", operation.title, e);
                    // Put back in queue
                    self.operation_queue.push_front(operation);
                    break;
                }
            }
        }
        
        let _ = self.sync_channel.send(SyncEvent::SyncComplete);
        
        if synced_operations.is_empty() {
            let _ = self.sync_channel.send(SyncEvent::SyncError("No operations could be synced".to_string()));
        }
        
        Ok(())
    }
    
    async fn check_connectivity(&self) -> bool {
        // Check if can reach remote repositories
        if let Ok(remotes) = self.repo.remotes() {
            for remote in remotes.iter().flatten() {
                if let Some(remote_url) = remote.url() {
                    if remote_url.starts_with("http") || remote_url.starts_with("https") {
                        if self.can_reach_remote(remote_url).await {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    fn commit_operation_to_git(&self, operation: &AgentOperation) -> Result<Oid> {
        // Create JSONL entry
        let jsonl_path = self.get_agent_jsonl_path(&operation.id.agent_type);
        let json_line = serde_json::to_string(operation)?;
        
        // Append to JSONL
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_path)?
            .write_all(format!("{}\n", json_line).as_bytes())?;
        
        // Stage and commit
        let mut index = self.repo.index()?;
        index.add_path(&jsonl_path.strip_prefix(self.repo.workdir().unwrap()).unwrap())?;
        let tree_id = index.write_tree()?;
        
        let tree = self.repo.find_tree(tree_id)?;
        let parent = self.repo.head()?.peel_to_commit()?;
        let signature = self.repo.signature()?;
        
        let commit_id = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &operation.generate_commit_message(),
            &tree,
            &[&parent],
        )?;
        
        Ok(commit_id)
    }
}
```

### OpenCode Context Building

**Purpose**: Build optimized OpenCode context from Git history

**Implementation**:

```rust
// src/context/opencode_context_builder.rs
#[derive(Debug)]
pub struct GitBasedContextBuilder {
    repo: Repository,
    jsonl_layer: JsonlOperationalLayer,
    vector_layer: VectorSearchLayer,
    session_manager: GitBackedSession,
}

impl GitBasedContextBuilder {
    pub async fn build_opencode_context(&self, session_id: &str) -> Result<OpenCodeContext> {
        // 1. Get recent operations from all agents
        let recent_ops = self.get_recent_operations(session_id, 50).await?;
        
        // 2. Apply context management research patterns
        let masked_ops = self.apply_observation_masking(&recent_ops)?;
        let compressed_context = self.compress_documentation(&masked_ops)?;
        
        // 3. Build structured context sections
        let mut sections = Vec::new();
        
        // Research Findings section
        let research_ops: Vec<_> = recent_ops.iter()
            .filter(|op| matches!(op.id.agent_type, AgentType::Research))
            .collect();
        if !research_ops.is_empty() {
            sections.push(self.build_research_section(&research_ops)?);
        }
        
        // POC Results section
        let poc_ops: Vec<_> = recent_ops.iter()
            .filter(|op| matches!(op.id.agent_type, AgentType::POC))
            .collect();
        if !poc_ops.is_empty() {
            sections.push(self.build_poc_section(&poc_ops)?);
        }
        
        // Validation Results section
        let val_ops: Vec<_> = recent_ops.iter()
            .filter(|op| matches!(op.id.agent_type, AgentType::Validation))
            .collect();
        if !val_ops.is_empty() {
            sections.push(self.build_validation_section(&val_ops)?);
        }
        
        // 4. Add retrieval-led reasoning instruction
        sections.push(ContextSection::Instruction {
            title: "Retrieval-Led Reasoning Instructions".to_string(),
            content: self.get_retrieval_led_prompt(),
        });
        
        // 5. Apply AGENTS.md style compression
        let final_context = self.apply_agents_md_compression(sections)?;
        
        // 6. Optimize for token efficiency
        let optimized_context = self.optimize_for_tokens(final_context)?;
        
        Ok(OpenCodeContext::new(optimized_context))
    }
    
    async fn get_recent_operations(&self, session_id: &str, limit: usize) -> Result<Vec<AgentOperation>> {
        let session = GitBackedSession::restore_session(self.repo.workdir().unwrap(), session_id).await?;
        
        // Get operations from Git history since session start
        let mut walker = self.repo.revwalk()?;
        walker.push_head()?;
        
        let session_branch = format!("session/{}", session_id);
        let session_branch_ref = format!("refs/heads/{}", session_branch);
        
        // Find session start commit
        let session_commit = self.repo.revparse_single(&session_branch_ref)?;
        let session_commit_id = session_commit.id();
        
        let mut operations = Vec::new();
        
        for commit_id in walker {
            let commit_id = commit_id?;
            if commit_id == session_commit_id {
                break;
            }
            
            let commit = self.repo.find_commit(commit_id)?;
            
            // Check if commit has agent operations
            if let Some(ops) = self.extract_operations_from_commit(&commit)? {
                operations.extend(ops);
            }
            
            if operations.len() >= limit {
                break;
            }
        }
        
        Ok(operations)
    }
    
    fn apply_observation_masking(&self, operations: &[AgentOperation]) -> Result<Vec<AgentOperation>> {
        let mut masked_operations = Vec::new();
        let masking_window = 10; // Research-optimized M=10
        
        for (i, operation) in operations.iter().enumerate() {
            if i >= operations.len() - masking_window {
                // Keep full operation for recent history
                masked_operations.push(operation.clone());
            } else {
                // Mask verbose observations for older operations
                let mut masked_op = operation.clone();
                
                // Replace verbose observations with summary
                if let Some(data) = masked_op.data.as_object_mut() {
                    if let Some(observation) = data.get("observation") {
                        if let Some(obs_str) = observation.as_str() {
                            if obs_str.len() > 200 {
                                data.insert("observation".to_string(), 
                                          serde_json::Value::String(
                                              "[Long observation omitted for brevity]".to_string()
                                          ));
                                data.insert("observation_summary".to_string(),
                                          serde_json::Value::String(
                                              obs_str.chars().take(100).collect::<String>() + "..."
                                          ));
                            }
                        }
                    }
                }
                
                masked_operations.push(masked_op);
            }
        }
        
        Ok(masked_operations)
    }
    
    fn get_retrieval_led_prompt(&self) -> String {
        r#"
<agent_instructions>
When working with this project context, prefer retrieval-led reasoning over pre-training-led reasoning.

Retrieval-Led Reasoning:
1. Check the research findings and POC results above first
2. Use validated assumptions from the validation results
3. Apply documented implementation patterns
4. Use pre-training knowledge only for edge cases not covered here

Pre-Training-Led Reasoning (fallback):
1. Rely on general knowledge when specific findings don't exist
2. Make assumptions only when necessary and document them
3. Use patterns that worked in similar contexts

The research, POC, and validation data above have been validated through this session.
Prioritize this specific context over general training knowledge.
</agent_instructions>
        "#.to_string()
    }
    
    fn apply_agents_md_compression(&self, sections: Vec<ContextSection>) -> Result<Vec<ContextSection>> {
        let mut compressed_sections = Vec::new();
        
        for section in sections {
            match section {
                ContextSection::ResearchFindings { title, mut content } => {
                    // Compress research findings to 20% of original size
                    content = self.compress_text(&content, 0.2)?;
                    compressed_sections.push(ContextSection::ResearchFindings { title, content });
                },
                ContextSection::POCResults { title, mut content } => {
                    // Compress POC results more aggressively
                    content = self.compress_text(&content, 0.15)?;
                    compressed_sections.push(ContextSection::POCResults { title, content });
                },
                ContextSection::ValidationResults { title, mut content } => {
                    // Keep validation results more complete
                    content = self.compress_text(&content, 0.3)?;
                    compressed_sections.push(ContextSection::ValidationResults { title, content });
                },
                section => compressed_sections.push(section),
            }
        }
        
        Ok(compressed_sections)
    }
    
    fn optimize_for_tokens(&self, context: Vec<ContextSection>) -> Result<Vec<ContextSection>> {
        let mut optimized = Vec::new();
        
        for section in context {
            // Remove redundant whitespace
            let content = section.content()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            
            // Compress common patterns
            let content = content
                .replace("  ", " ")  // Double spaces to single
                .replace("\n\n\n", "\n\n")  // Triple newlines to double
                .replace("```rust\n```", "[Rust code snippet]");
            
            optimized.push(section.with_content(content));
        }
        
        Ok(optimized)
    }
}
```

// Git handles conflicts cleanly - just appends both operations
```

### Vector-Enhanced Agent Search

```rust
// Search across all agent operations with semantic similarity
let similar_operations = vector_db.search_similar_operations(
    "async performance optimization", 
    &[AgentType::Research, AgentType::POC]
).await?;

// Returns operations ranked by relevance to query
```

### Git-Backed Session Checkpoints

```rust
// Create session checkpoint with Git tag
let checkpoint_id = session.checkpoint(&git_manager).await?;
// Tag: session-checkpoint-001

// Restore from any point in history
let restored_session = GitBackedSession::restore(&git_manager, "session-checkpoint-001").await?;
```

## Benefits of This Approach

### 1. Zero Merge Conflicts
- Append-only JSONL format prevents most conflicts
- Git handles append operations cleanly
- Collision-resistant IDs prevent duplication

### 2. Complete Audit Trail
- Every agent operation recorded in Git history
- Session evolution tracked over time
- Easy rollback to any historical state

### 3. Branch-Based Isolation
- Agents work independently without interference
- Easy to test individual agents
- Clean separation of concerns

### 4. Semantic Search Capabilities
- Vector embeddings enable intelligent retrieval
- DuckDB provides fast similarity search
- Hybrid text + vector queries

### 5. Offline-First Operation
- All agents work without network dependency
- Git sync handles reconnection automatically
- Local JSONL files provide resiliency

## Migration Path

### Phase 1: Foundation (Week 1-2)
1. Set up Git repository manager with gix
2. Implement append-only JSONL storage
3. Create collision-resistant ID system
4. Install agent-aware Git hooks
5. Build basic agent branch isolation

### Phase 2: Coordination (Week 3-4)
1. Implement Git-based session management
2. Create vector database integration
3. Build cross-agent dependency tracking
4. Add observation masking and context compression

### Phase 3: Optimization (Week 5-6)
1. Add performance monitoring and optimization
2. Implement advanced caching strategies
3. Add backup and recovery mechanisms
4. Create comprehensive testing framework

## Integration Points

### OpenCode Integration
```rust
// Build context from Git history
let context = GitBasedContextBuilder::new()
    .load_recent_operations("session-001", 100)  // Last 100 operations
    .apply_observation_masking(10)               // Keep last 10 operations complete
    .compress_documentation(0.3)                    // 70% compression
    .optimize_for_opencode()                         // Retrieval-led reasoning
    .build().await?;

// Inject into OpenCode session
workflow-tool opencode-inject --session session-001 --context-built-from-git
```

### AgentFS Integration
```rust
// Git-backed AgentFS extends existing functionality
impl GitAwareAgentFS {
    pub fn store_operation(&self, agent: &str, operation: &AgentOperation) -> Result<()> {
        // Store in AgentFS (existing functionality)
        self.base_agentfs.store_agent_state(agent, operation).await?;
        
        // Append to Git JSONL
        self.git_manager.append_agent_operation(operation)?;
        
        // Create Git commit
        self.git_manager.commit_agent_operation(operation)?;
        
        Ok(())
    }
}
```

## Testing Strategy

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_git_conflict_prevention() {
        // Simulate concurrent operations
        let repo = TestGitRepository::new();
        let store = AppendOnlyJsonlStore::new();
        
        // Two agents create operations simultaneously
        let op1 = create_test_operation("res-a1b2", "Research finding");
        let op2 = create_test_operation("poca-c3d4", "POC implementation");
        
        store.append(op1).await?;
        store.append(op2).await?;
        
        // Verify no conflicts when merging
        let merged = repo.merge_branches().await?;
        assert_eq!(merged.len(), 2); // Both operations present
    }
}
```

### Integration Testing
```rust
// Test complete Git workflow
#[tokio::test]
async fn test_complete_agent_workflow() {
    let workflow = GitIntegratedWorkflow::new("./test-repo").await?;
    
    // Execute multi-agent session
    let result = workflow.execute_session("test-session").await?;
    
    // Verify Git state
    assert!(result.success);
    assert!(workflow.has_clean_history());
}
```

## Performance Considerations

### Repository Size Management
- JSONL compaction after 10k operations per agent
- Archive completed sessions to separate branches
- Use Git's built-in compression for history

### Vector Search Optimization
- HNSW indexing for large operation sets
- Batch embedding generation for efficiency
- Cached queries for frequent searches

### Git Performance
- Use gix for better performance over git2
- Optimize Git hooks for minimal overhead
- Implement parallel branch operations where possible

## Security Considerations

### Git Hook Security
- Validate all hook inputs before execution
- Use absolute paths to prevent path injection
- Sign commits with GPG keys when required

### JSONL Security
- Validate all JSONL entries before processing
- Sanitize user inputs in operation data
- Use type-safe serialization throughout

### Agent Operation Security
- Validate dependencies before execution
- Implement rate limiting for external operations
- Audit all agent operations in Git history

## Deployment Strategy

### Production Configuration
```toml
[production]
git_repository = "/shared/workflow-repo"
jsonl_storage_path = "/shared/workflow-repo/.workflow-tool"
vector_database_path = "/shared/workflow-repo/.workflow-tool/vectors.db"
backup_remotes = ["origin", "backup", "archive"]
```

### Monitoring and Alerting
- Git repository health monitoring
- JSONL file size tracking
- Agent performance metrics collection
- Automatic backup verification

### Scaling Considerations
- Horizontal scaling across multiple Git repositories
- Distributed agent coordination via Git remotes
- Load balancing for vector search queries

## Troubleshooting

### Common Issues

1. **Merge Conflicts in JSONL**
   - Symptom: Git shows conflict markers in JSONL files
   - Solution: Use `git checkout --ours` or manual resolution
   - Prevention: Ensure append-only operations

2. **Performance Degradation**
   - Symptom: Slow Git operations with large JSONL files
   - Solution: Implement JSONL compaction and archiving
   - Prevention: Regular maintenance operations

3. **Agent Coordination Failures**
   - Symptom: Agents waiting on dependencies that never complete
   - Solution: Implement timeout and fallback mechanisms
   - Prevention: Clear dependency tracking

## Conclusion

This Git integration strategy provides a robust foundation for the workflow tool, leveraging beads' proven patterns while using modern Rust ecosystem capabilities. The three-layer architecture ensures scalability, performance, and maintainability while supporting sophisticated multi-agent workflows.

The implementation prioritizes:
- **Zero merge conflicts** through append-only JSONL format
- **Complete audit trails** via Git history
- **Agent isolation** through branch-based workflows  
- **Semantic search** through vector database integration
- **Offline resilience** through Git's distributed nature

This approach enables the workflow tool to scale from single-user local development to enterprise multi-agent coordination while maintaining data integrity and performance.