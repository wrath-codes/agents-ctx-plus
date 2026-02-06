# Data Management Strategy

## 💾 Storage Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    DATA MANAGEMENT ARCHITECTURE                │
│                                                             │
│  ┌─────────────────┐    ┌─────────────────┐               │
│  │    Local       │    │     Cloud       │               │
│  │                │    │                 │               │
│  │ • AgentFS      │    │ • Cloudflare    │               │
│  │ • DuckDB+VSS   │    │   R2 Storage    │               │
│  │ • Local Cache   │    │ • Global Index   │               │
│  │ • Config Files  │    │ • Sync Service   │               │
│  └─────────────────┘    └─────────────────┘               │
│           │                      │                         │
│           ▼                      ▼                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              Hybrid Sync Strategy                ││
│  │                                                     ││
│  │ • Metadata: Real-time sync                            ││
│  │ • Small Files: Auto-sync (<100KB)                    ││
│  │ • Large Files: Intelligent sync                          ││
│  │ • User Data: Opt-in sharing                           ││
│  │ • Performance: Usage-based sync                         ││
│  └─────────────────────────────────────────────────────────────────────┘│
│                                                             │
│           ▼                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │            Privacy & Security Architecture           ││
│  │                                                        ││
│  │ • Local-First Processing                             ││
│  │ • Configurable Boundaries                            ││
│  │ • Encryption at Rest                                 ││
│  │ • Access Control & Audit                             ││
│  │ • Opt-in Data Sharing                                ││
│  └─────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

---

## 🗄️ Storage Components

### 1. **AgentFS: Agent State Management**

**Primary Use**: Agent coordination, session tracking, audit trails  
**Key Features**:

- Automatic agent state persistence
- Complete audit trail of all operations
- Multi-agent coordination and conflict resolution
- Session restoration and backup
- Privacy boundary management

**Schema Design**:

```sql
-- AgentFS core tables
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL,           -- research, poc, documentation, validation
    capabilities JSON,            -- Available tools and their configurations
    status TEXT DEFAULT 'inactive', -- inactive, active, busy, error
    created_at TIMESTAMP,
    last_activity TIMESTAMP
);

CREATE TABLE agent_states (
    id TEXT PRIMARY KEY,
    agent_id TEXT REFERENCES agents(id),
    state_key TEXT NOT NULL,
    state_value JSON,
    updated_at TIMESTAMP,
    version INTEGER DEFAULT 1
);

CREATE TABLE agent_communications (
    id TEXT PRIMARY KEY,
    from_agent TEXT REFERENCES agents(id),
    to_agent TEXT REFERENCES agents(id),
    message_type TEXT,
    message_content JSON,
    timestamp TIMESTAMP,
    status TEXT DEFAULT 'delivered'
);
```

**Benefits**:

- **Built-in audit trails** for all agent actions
- **Automatic conflict resolution** with supervisor arbitration
- **Session persistence** across restarts
- **Multi-device sync** with privacy controls

### 2. **DuckDB + VSS: Vector Search Engine**

**Primary Use**: High-performance vector similarity search and metadata queries  
**Key Features**:

- Native vector search with HNSW indexing
- SQL + Vector hybrid queries
- Real-time search performance
- Metadata filtering and complex queries

**Vector Search Schema**:

```sql
-- Enable VSS extension
INSTALL vss;
LOAD vss;

-- Documents table with vector search
CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT,
    embedding FLOAT[384],           -- FastEmbed 384-dim vectors
    metadata JSON,                 -- Language, library, version, etc.
    r2_location TEXT NOT NULL,       -- Reference to actual file
    file_size_bytes INTEGER,
    checksum TEXT,
    created_at TIMESTAMP,
    access_count INTEGER DEFAULT 0,
    last_accessed TIMESTAMP
);

-- Create HNSW index for fast search
CREATE INDEX doc_vector_idx ON documents USING HNSW (embedding);

-- Hybrid search example
SELECT id, title, metadata, array_cosine_similarity(embedding, ?) as similarity
FROM documents
WHERE metadata->>'language' = 'rust'
  AND metadata->>'library' = 'tokio'
ORDER BY similarity DESC
LIMIT 10;
```

**Performance Characteristics**:

- **Sub-second search** across millions of documents
- **Hybrid queries** combining text search + vector similarity
- **Real-time indexing** for new documents
- **Efficient filtering** by metadata fields

### 3. **Cloudflare R2: Scalable Document Storage**

**Primary Use**: Long-term document storage and global documentation index  
**Key Features**:

- Unlimited storage capacity
- Global CDN distribution
- Cost-effective egress pricing
- S3-compatible API

**Storage Structure**:

```
s3://workflow-docs/
├── libraries/                    # Community documentation index
│   ├── rust/
│   │   ├── serde/
│   │   │   ├── v1.0.200/
│   │   │   │   ├── serde.json          -- Parsed API docs
│   │   │   │   ├── examples/           -- Code examples
│   │   │   │   └── metadata.json      -- Library metadata
│   │   │   └── v1.0.199/
│   │   └── tokio/
│   ├── python/
│   │   ├── fastapi/
│   │   └── django/
│   └── typescript/
│       ├── react/
│       └── nextjs/
├── user-contributions/           # User-uploaded custom docs
│   ├── user-123/
│   │   └── custom-libs/
│   └── user-456/
└── temp/                       # Temporary processing files
```

**Cost Optimization**:

- **Intelligent compression** before upload
- **Deduplication** to avoid storing duplicates
- **Lifecycle management** with automatic cleanup
- **Edge caching** for frequently accessed documents

### 4. **Local Cache: Multi-Tier Performance**

**Primary Use**: Performance optimization and offline capability  
**Key Features**:

- L1: In-memory cache for current session
- L2: AgentFS persistent cache for recent data
- L3: Local filesystem cache for downloaded documents

**Cache Hierarchy**:

```rust
pub struct MultiTierCache {
    l1_cache: Arc<RwLock<LruCache<String, CachedData>>>,  // Session memory
    l2_cache: AgentFSCache,                               // AgentFS storage
    l3_cache: LocalFileSystemCache,                           // File system
}

impl MultiTierCache {
    pub async fn get(&self, key: &str) -> Option<CachedData> {
        // L1: Fastest, session-scoped
        if let Some(data) = self.l1_cache.read().await.get(key) {
            return Some(data);
        }

        // L2: Fast, persistent
        if let Ok(Some(data)) = self.l2_cache.get(key).await {
            self.l1_cache.write().await.put(key.to_string(), data.clone());
            return Some(data);
        }

        // L3: Slower, but comprehensive
        if let Some(data) = self.l3_cache.get(key).await {
            // Promote to higher tiers
            self.l2_cache.put(key, &data).await?;
            self.l1_cache.write().await.put(key.to_string(), data.clone());
            return Some(data);
        }

        None
    }
}
```

---

## 🔒 Privacy & Security Architecture

### Privacy Boundaries Configuration

**Configurable Privacy Levels**:

```yaml
# privacy_config.yml
privacy:
  default_level: "private_until_shared"

  boundaries:
    # Automatic classification patterns
    private_patterns:
      - "api_keys"
      - "credentials"
      - "personal_notes"
      - "local_secrets"
      - "ssh_keys"

    # User-defined boundaries
    custom_private_patterns:
      - "internal/*"
      - "proprietary/*"

  sharing_controls:
    # Granular opt-in controls
    research_findings:
      default: "anonymous" # Strip personal data
      allow_opt_in: true

    code_examples:
      default: "team_only" # Only with team members
      allow_opt_in: false

    implementation_logs:
      default: "private" # Never auto-share
      allow_opt_in: false

    assumptions:
      default: "team_opt_out" # Team members can opt-out
      allow_opt_in: true
```

### Security Measures

**Data Protection**:

```rust
pub struct SecurityManager {
    encryption_key: EncryptionKey,
    access_control: AccessController,
    audit_logger: AuditLogger,
}

impl SecurityManager {
    // Encryption at rest
    pub async fn encrypt_sensitive_data(&self, data: &[u8]) -> Result<EncryptedData> {
        let encrypted = self.encryption_key.encrypt(data)?;
        Ok(EncryptedData::new(encrypted))
    }

    // Access control
    pub fn check_access(&self, user_id: &str, resource: &Resource) -> AccessDecision {
        let permissions = self.access_control.get_permissions(user_id);
        match resource.privacy_level {
            PrivacyLevel::Public => AccessDecision::Allow,
            PrivacyLevel::Team => {
                if permissions.team_access { AccessDecision::Allow }
                else { AccessDecision::Deny("No team access") }
            },
            PrivacyLevel::Private => {
                if permissions.owner == user_id { AccessDecision::Allow }
                else { AccessDecision::Deny("Private resource") }
            },
        }
    }

    // Comprehensive audit logging
    pub fn log_access_attempt(&self, user_id: &str, resource: &str, decision: &AccessDecision) {
        self.audit_logger.log(AuditEvent {
            user_id: user_id.to_string(),
            resource: resource.to_string(),
            decision: decision.clone(),
            timestamp: Utc::now(),
            ip_address: self.get_client_ip(),
        });
    }
}
```

---

## 🔄 Sync Strategy

### Intelligent Sync Algorithm

```rust
pub struct IntelligentSync {
    local_agentfs: AgentFS,
    cloud_sync: CloudSyncService,
    network_monitor: NetworkMonitor,
}

impl IntelligentSync {
    pub async fn sync_cycle(&self) -> Result<SyncResult> {
        let sync_strategy = self.determine_sync_strategy();

        match sync_strategy {
            SyncStrategy::Realtime => {
                // High-value users with stable connection
                self.sync_immediate_changes().await?;
                self.continuous_sync().await?;
            },
            SyncStrategy::Adaptive => {
                // Balance performance and freshness
                self.sync_prioritized_changes().await?;
                self.schedule_background_sync().await?;
            },
            SyncStrategy::OnDemand => {
                // Cost-conscious or offline users
                self.sync_on_user_request().await?;
            }
        }

        Ok(SyncResult::new(sync_strategy, self.get_sync_stats()))
    }

    fn determine_sync_strategy(&self) -> SyncStrategy {
        let user_tier = self.get_user_tier();
        let network_quality = self.network_monitor.get_quality();
        let data_volume = self.estimate_sync_size();

        match (user_tier, network_quality, data_volume) {
            (UserTier::Pro, NetworkQuality::Excellent, _) => SyncStrategy::Realtime,
            (UserTier::Pro, NetworkQuality::Good, _) => SyncStrategy::Adaptive,
            (_, _, DataVolume::High) => SyncStrategy::OnDemand,
            (UserTier::Free, _, _) => SyncStrategy::Adaptive,
        }
    }
}
```

### Conflict Resolution

**Optimistic Concurrency with Supervisor Arbitration**:

```rust
pub enum ConflictResolution {
    // Auto-resolve conflicts
    AutoMerge {
        non_conflicting_edits: MergeRule::AcceptAll,
        complementary_changes: MergeRule::IntelligentMerge,
        tool_boundary_conflicts: MergeRule::SupervisorDecides,
    },

    // User intervention required
    UserArbitration {
        same_file_conflicts: bool,
        assumption_conflicts: bool,
        critical_data_changes: bool,
    },
}

pub struct ConflictResolver {
    strategy: ConflictResolution,
    supervisor: SupervisorAgent,
}

impl ConflictResolver {
    pub async fn resolve_conflict(&self, conflict: &Conflict) -> Result<Resolution> {
        match conflict.type {
            ConflictType::ToolBoundary => {
                // Supervisor decides based on agent responsibilities
                let decision = self.supervisor.arbitrate_tool_conflict(conflict).await?;
                Ok(Resolution::SupervisorDecision(decision))
            },
            ConflictType::DataConflict => {
                // Try auto-merge first
                if let Some(merge_result) = self.attempt_auto_merge(conflict) {
                    Ok(Resolution::AutoMerged(merge_result))
                } else {
                    self.request_user_arbitration(conflict).await
                }
            },
        }
    }
}
```

---

## 📊 Performance Optimization

### Hardware-Adaptive Resource Management

```rust
pub struct AdaptiveResourceManager {
    hardware_profile: HardwareProfile,
    current_load: SystemLoad,
    resource_limits: ResourceLimits,
}

impl AdaptiveResourceManager {
    pub fn detect_hardware_capabilities(&self) -> HardwareProfile {
        HardwareProfile {
            cpu_cores: num_cpus::get(),
            available_memory_gb: get_available_memory() / 1024.0 / 1024.0,
            has_gpu: check_gpu_availability(),
            storage_speed_mbps: benchmark_storage_speed(),
            network_speed_mbps: benchmark_network_speed(),
        }
    }

    pub fn calculate_optimal_limits(&self) -> ResourceLimits {
        match (self.hardware_profile.cpu_cores, self.hardware_profile.available_memory_gb) {
            (0..=2, 0..=8) => ResourceLimits {
                max_concurrent_agents: 1,
                memory_per_agent_mb: 512,
                cache_size_mb: 256,
            },
            (3..=4, 8..=16) => ResourceLimits {
                max_concurrent_agents: 2,
                memory_per_agent_mb: 1024,
                cache_size_mb: 512,
            },
            (5..=8, 16..=32) => ResourceLimits {
                max_concurrent_agents: 4,
                memory_per_agent_mb: 2048,
                cache_size_mb: 1024,
            },
            (_, _) => ResourceLimits {
                max_concurrent_agents: 8,
                memory_per_agent_mb: 4096,
                cache_size_mb: 2048,
            },
        }
    }
}
```

---

## 🔗 Cross-References

- Related to: [Architecture Overview](./01-architecture-overview.md#storage-layer)
- Related to: [Agent System Design](./02-agent-system-design.md#agent-coordination)
- Related to: [OpenCode Integration](./04-opencode-integration.md#data-sharing)
- Related to: [Task List](./TASKLIST.md#phase-3-data-management)

