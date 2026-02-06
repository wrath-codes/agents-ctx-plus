# Data Flow

This document provides complete flow diagrams and explanations of how data moves through Beads' three-layer architecture during all operations.

## 🔄 Overview of Data Flow

Beads' data flow follows a clear pattern across its three layers:

```
flowchart TD
    subgraph "User Interface"
        CLI[CLI Commands]
        Hooks[Git Hooks]
        Agent[AI Agent Integration]
    end
    
    subgraph "Layer 3: SQLite (Fast Queries)"
        SQLite[(beads.db)]
        Indexes[(Indexes)]
        Cache[(Query Cache)]
    end
    
    subgraph "Layer 2: JSONL (Operational)"
        JSONL[(issues.jsonl)]
        Interactions[(interactions.jsonl)]
        Routes[(routes.jsonl)]
        Config[(config.yaml)]
    end
    
    subgraph "Layer 1: Git (Historical)"
        GitRepo[Git Repository]
        Remote[Remote Repository]
        Branches[Branches & Merges]
    end
    
    CLI -->|Read| SQLite
    CLI -->|Write| SQLite
    SQLite -->|Append| JSONL
    JSONL -->|Rebuild| SQLite
    
    JSONL <-->|Commit/Pull| GitRepo
    GitRepo <-->|Push/Pull| Remote
    GitRepo -->|Switch| Branches
    
    Hooks -->|Trigger| CLI
    Agent -->|JSON Output| CLI
    Agent -->|Session Events| Hooks
    
    style CLI fill:#e1f5fe
    style SQLite fill:#f3e5f5
    style JSONL fill:#e8f5e8
    style GitRepo fill:#e0f2f1
```

## 📝 Write Operations Flow

### Create Issue Operation

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant SQLite
    participant JSONL
    participant Git
    
    User->>CLI: bd create "New issue" -p 1
    CLI->>SQLite: INSERT INTO issues
    SQLite->>CLI: Return new issue ID
    CLI->>JSONL: Append create operation
    Note over JSONL: {"id": "bd-a1b2", "type": "create", ...}
    CLI->>Git: git add issues.jsonl (async)
    CLI->>User: Return issue ID immediately
    Note over User: No waiting for git commit
    
    Daemon->>Git: git commit (5s debounce)
    Daemon->>Remote: git push (async)
```

### Update Issue Operation

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant SQLite
    participant JSONL
    participant Daemon
    
    User->>CLI: bd update bd-a1b2 --status in_progress
    CLI->>SQLite: UPDATE issues SET status
    CLI->>JSONL: Append update operation
    Note over JSONL: {"id": "bd-a1b2", "type": "update", ...}
    CLI->>User: Return success immediately
    
    Daemon->>Daemon: File watcher detects change
    Daemon->>Daemon: Start 5-second debounce timer
    Daemon->>Git: After debounce: git commit
    Daemon->>Remote: git push
```

### Add Dependency Operation

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant SQLite
    participant JSONL
    
    User->>CLI: bd dep add bd-c3d4 bd-a1b2
    CLI->>SQLite: INSERT INTO dependencies
    CLI->>JSONL: Append dependency operation
    Note over JSONL: {"type": "dependency", "data": {"action": "add", ...}}
    
    Note over SQLite: Update indexes for fast queries
    CLI->>User: Return success
    
    Daemon->>Daemon: Schedule sync (debounced)
```

## 📖 Read Operations Flow

### List Issues Query

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant SQLite
    participant Indexes
    participant Cache
    
    User->>CLI: bd list --status open --priority 0,1
    CLI->>Cache: Check cached results
    alt Cache hit
        Cache->>CLI: Return cached issues
    else Cache miss
        CLI->>SQLite: SELECT with WHERE clause
        SQLite->>Indexes: Use status+priority index
        Indexes->>SQLite: Return matching row IDs
        SQLite->>Indexes: Fetch full row data
        SQLite->>CLI: Return issue rows
        CLI->>Cache: Store result in cache
    end
    CLI->>User: Return formatted list
```

### Ready Work Calculation

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant SQLite
    participant DepsTable
    participant IssuesTable
    
    User->>CLI: bd ready
    CLI->>SQLite: Complex ready work query
    SQLite->>IssuesTable: SELECT open/in_progress issues
    IssuesTable->>SQLite: Return candidate issues
    SQLite->>DepsTable: Check blockers for each
    Note over DepsTable: WHERE type = 'blocks' AND parent_status != 'closed'
    DepsTable->>SQLite: Return blocked issue IDs
    SQLite->>SQLite: Filter candidates removing blocked
    SQLite->>CLI: Return unblocked issues
    CLI->>User: Display ready work (ordered by priority)
```

### Show Issue Details

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant SQLite
    participant DepsTable
    participant CommentsTable
    
    User->>CLI: bd show bd-a1b2
    CLI->>SQLite: SELECT * FROM issues WHERE id = 'bd-a1b2'
    SQLite->>CLI: Return issue details
    CLI->>DepsTable: SELECT dependencies for issue
    DepsTable->>CLI: Return dependency list
    CLI->>CommentsTable: SELECT comments for issue
    CommentsTable->>CLI: Return comment history
    CLI->>User: Format and display complete issue
```

## 🔄 Sync Operations Flow

### Standard Sync Operation

```mermaid
flowchart TD
    Start([User runs bd sync]) --> CheckRemote{Remote changes?}
    
    CheckRemote -->|Yes| Pull[git pull]
    Pull --> MergeJSONL[Merge JSONL files]
    MergeJSONL --> RebuildSQLite[Rebuild SQLite from JSONL]
    
    CheckRemote -->|No| CheckLocal{Local changes?}
    CheckLocal -->|Yes| CommitJSONL[git add .beads/issues.jsonl]
    CommitJSONL --> GitCommit[git commit]
    
    CheckLocal -->|No| CheckPush{Need to push?}
    RebuildSQLite --> CheckPush
    GitCommit --> CheckPush
    
    CheckPush -->|Yes| Push[git push]
    Push --> End([Sync complete])
    
    CheckPush -->|No| End
```

### Daemon-Triggered Sync

```mermaid
stateDiagram-v2
    [*] --> FileChange: File modified
    FileChange --> Debounce: Start 5s timer
    
    Debounce --> AnotherChange: Another file change
    AnotherChange --> Debounce: Reset timer
    
    Debounce --> SyncTrigger: Timer expires
    SyncTrigger --> AcquireLock: Get database lock
    AcquireLock --> GitPull: Check remote
    GitPull --> MergeJSONL: Merge if needed
    MergeJSONL --> RebuildDB: Rebuild SQLite
    RebuildDB --> GitCommit: Commit changes
    GitCommit --> GitPush: Push to remote
    GitPush --> ReleaseLock: Release lock
    ReleaseLock --> [*]: Wait for next change
```

### Import-Only Sync

```mermaid
flowchart TD
    Start([bd sync --import-only]) --> KillDaemon[Stop daemon]
    KillDaemon --> RemoveDB[Remove beads.db*]
    RemoveDB --> ReadJSONL[Read issues.jsonl]
    ReadJSONL --> ProcessOps[Process operations sequentially]
    ProcessOps --> CreateTables[Create fresh SQLite]
    CreateTables --> InsertData[Apply operations to SQLite]
    InsertData --> CreateIndexes[Build performance indexes]
    CreateIndexes --> End([SQLite rebuilt])
```

## 🔀 Multi-Agent Data Flow

### Cross-Repository Dependencies

```mermaid
flowchart TD
    subgraph "Repo A: frontend"
        CLI_A[bd create "API integration"]
        JSONL_A[issues.jsonl]
        Dep_A[external:backend-repo/bd-xyz]
    end
    
    subgraph "Repo B: backend"  
        CLI_B[bd dep add bd-local Dep_A]
        JSONL_B[issues.jsonl]
        Routes_B[routes.jsonl]
    end
    
    subgraph "Sync Process"
        Git_A[Git push Repo A]
        Git_B[Git push Repo B]
        Hydrate[bd hydrate --from backend-repo]
    end
    
    CLI_A --> JSONL_A
    JSONL_A --> Dep_A
    Dep_A --> CLI_B
    CLI_B --> JSONL_B
    Routes_B --> Hydrate
    
    JSONL_A --> Git_A
    JSONL_B --> Git_B
    Git_A --> Hydrate
    Git_B --> Hydrate
```

### Agent Handoff Pattern

```mermaid
sequenceDiagram
    participant AgentA
    participant Issue
    participant JSONL
    participant Git
    participant AgentB
    
    AgentA->>Issue: Complete task
    AgentA->>Issue: bd close bd-a1b2 --reason "Ready for review"
    Issue->>JSONL: Append close operation
    Issue->>JSONL: Append comment "Handed off to AgentB"
    AgentA->>Issue: bd pin bd-a1b2 --for agent-b
    Issue->>JSONL: Update assignee
    
    Daemon->>Git: Commit changes
    Daemon->>Git: Push to remote
    
    AgentB->>Git: Pull latest changes
    AgentB->>Issue: bd hook --agent agent-b
    Issue->>AgentB: Show assigned issue
    AgentB->>Issue: bd update bd-a1b2 --status in_progress
```

## 🌐 Integration Data Flow

### Claude Code Integration

```mermaid
sequenceDiagram
    participant Claude
    participant Hooks[Claude Hooks]
    participant CLI
    participant SQLite
    participant JSONL
    
    Note over Claude,Claude Hooks: Session Start
    Claude->>Hooks: SessionStart event
    Hooks->>CLI: bd prime
    CLI->>SQLite: Query ready work
    CLI->>JSONL: Read recent interactions
    CLI->>Claude: Return ~1-2k tokens context
    
    Note over Claude,Claude Hooks: During Session
    Claude->>CLI: bd commands as needed
    CLI->>SQLite: Immediate responses
    
    Note over Claude,Claude Hooks: Session End
    Claude->>Hooks: PreCompact event
    Hooks->>CLI: bd sync
    CLI->>JSONL: Ensure all operations appended
    CLI->>Git: Commit and push changes
```

### MCP Server Integration

```mermaid
flowchart TD
    subgraph "MCP Client"
        Claude[Claude Desktop]
        VSCode[VS Code]
        Cursor[Cursor]
    end
    
    subgraph "MCP Protocol"
        MCP[MCP Server]
        Protocol[JSON-RPC Messages]
    end
    
    subgraph "Beads Backend"
        CLI[bd CLI commands]
        SQLite[(beads.db)]
        JSONL[(issues.jsonl)]
    end
    
    Claude --> Protocol
    VSCode --> Protocol
    Cursor --> Protocol
    
    Protocol --> MCP
    MCP --> CLI
    CLI --> SQLite
    CLI --> JSONL
    
    SQLite --> MCP
    JSONL --> MCP
    MCP --> Protocol
    Protocol --> Claude
    Protocol --> VSCode
    Protocol --> Cursor
```

## 🛡️ Error Handling Flow

### Database Corruption Recovery

```mermaid
stateDiagram-v2
    [*] --> Corruption: Database error detected
    Corruption --> StopDaemon: bd daemons killall
    StopDaemon --> RemoveDB: Remove corrupted files
    RemoveDB --> Rebuild: bd sync --import-only
    Rebuild --> Success: Rebuild successful?
    Success -->|Yes| Running: Daemon monitoring
    Success -->|No| ManualFix: Manual intervention required
    ManualFix --> [*]
    
    Running --> [*]: Normal operation
```

### Git Conflict Resolution

```mermaid
flowchart TD
    Start([Git merge conflict]) --> CheckType{Conflict type?}
    
    CheckType -->|JSONL syntax| ParseError[Invalid JSONL]
    ParseError --> ManualEdit[Edit file manually]
    ManualEdit --> Validate[bd check --jsonl]
    Validate --> Success{Valid?}
    Success -->|Yes| Commit[git add .beads/issues.jsonl]
    Success -->|No| ManualEdit
    
    CheckType -->|Same issue lines| MergeConflict[Multiple lines for same issue]
    MergeConflict --> KeepBoth[Keep all operations]
    KeepBoth --> Rebuild[bd sync --import-only]
    Rebuild --> End([Conflict resolved])
    
    Commit --> Push[git push]
    Push --> End
```

## 📊 Performance Flow Analysis

### Query Optimization Flow

```mermaid
flowchart TD
    Query[User query] --> Parse[Parse SQL]
    Parse --> Plan[Query planner analyzes]
    Plan --> Index{Appropriate index?}
    
    Index -->|Yes| IndexScan[Index range scan]
    Index -->|No| TableScan[Full table scan]
    
    IndexScan --> Filter[Apply filters]
    TableScan --> Filter
    Filter --> Sort[Sort if needed]
    Sort --> Limit[Apply limit]
    Limit --> Return[Return results]
    
    Return --> Cache[Store in query cache]
    
    Note over IndexScan: milliseconds for 10k issues
    Note over TableScan: seconds for 10k issues
```

### Memory Management Flow

```mermaid
stateDiagram-v2
    [*] --> LowUsage: Memory < 50MB
    LowUsage --> Normal: Optimize for speed
    Normal --> MediumUsage: Memory 50-100MB
    MediumUsage --> Conservative: Reduce cache sizes
    Conservative --> HighUsage: Memory > 100MB
    HighUsage --> Minimal: Minimum memory usage
    Minimal --> Critical: Memory > 200MB
    Critical --> Emergency: Stop operations
    Emergency --> [*]: Alert user
```

## 🔗 Multi-Repository Flow

### Hydration Process

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Routes
    participant RemoteRepos[Remote Repositories]
    participant LocalJSONL
    
    User->>CLI: bd hydrate --from backend-repo
    CLI->>Routes: Check routes configuration
    Routes->>CLI: Return backend-repo URL
    CLI->>RemoteRepos: git clone/pull backend-repo
    RemoteRepos->>CLI: Provide issues.jsonl
    CLI->>LocalJSONL: Merge external issues
    Note over LocalJSONL: Mark as external:repo/issue
    CLI->>SQLite: Rebuild with new issues
    CLI->>User: Hydration complete
```

### Cross-Repo Dependency Updates

```mermaid
flowchart TD
    RepoA[Repo A: frontend] --> DepUpdate[Create/update dependency]
    DepUpdate --> CheckRoute{Route exists?}
    CheckRoute -->|Yes| UpdateLocal[Update local JSONL]
    CheckRoute -->|No| CreateRoute[Create routing rule]
    
    UpdateLocal --> SyncRepoA[Sync Repo A]
    CreateRoute --> SyncRepoA
    SyncRepoA --> SyncRepoB[Notify Repo B]
    SyncRepoB --> HydrateTarget[Repo B hydrates from A]
    HydrateTarget --> Complete[Dependencies synchronized]
```

## 🔍 Monitoring Flow

### Daemon Health Monitoring

```mermaid
flowchart TD
    Start([Daemon health check]) --> CheckProcess{Process running?}
    CheckProcess -->|No| Stopped[Daemon stopped]
    Stopped --> Alert[Alert user]
    
    CheckProcess -->|Yes| CheckDB{Database accessible?}
    CheckDB -->|No| DBError[Database error]
    DBError --> Recovery[Attempt recovery]
    
    CheckDB -->|Yes| CheckFiles{Files accessible?}
    CheckFiles -->|No| FileError[File system error]
    CheckFiles -->|Yes| CheckResources{Resource limits OK?}
    
    CheckResources -->|No| ResourceWarning[Resource limit exceeded]
    CheckResources -->|Yes| Healthy[All systems healthy]
    
    Recovery --> Healthy
    ResourceWarning --> Healthy
    Healthy --> End([Health check complete])
    Alert --> End
    DBError --> End
    FileError --> End
```

### Performance Metrics Collection

```mermaid
sequenceDiagram
    participant Operations
    participant Metrics
    participant Storage
    participant Alerts
    
    Note over Operations,Alerts: Continuous monitoring
    Operations->>Metrics: Record operation time
    Operations->>Metrics: Record memory usage
    Operations->>Metrics: Record error count
    
    Metrics->>Metrics: Calculate aggregates
    Metrics->>Metrics: Compare with thresholds
    
    alt Threshold exceeded
        Metrics->>Alerts: Trigger alert
        Alerts->>Storage: Log performance issue
    else Normal operation
        Metrics->>Storage: Log normal metrics
    end
```

## 🔗 Related Documentation

- [Architecture Overview](overview.md) - Three-layer system context
- [Git Layer](git-layer.md) - Historical data flow
- [JSONL Layer](jsonl-layer.md) - Operational format details
- [SQLite Layer](sqlite-layer.md) - Database operations
- [Daemon System](daemon-system.md) - Background sync processes

## 📚 See Also

- [CLI Reference](../cli-reference/) - Command-specific flows
- [Multi-Agent](../multi-agent/) - Multi-agent data flows
- [Integrations](../integrations/) - Integration data flows
- [Recovery](../recovery/) - Error handling and recovery flows