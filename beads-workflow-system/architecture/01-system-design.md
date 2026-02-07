# System Design Overview

## 🏗️ Architecture Components

### Core System Architecture

```
┌─────────────────────────────────────────────────────────┐
│              WORKFLOW SYSTEM ARCHITECTURE          │
│                                                     │
│  ┌─────────────────┐    ┌─────────────────┐         │
│  │   BEADS CORE    │    │  TEMPOLITE CORE │         │
│  │                 │    │                 │         │
│  │ • Issues       │    │ • Activities    │         │
│  │ • Dependencies  │    │ • Sagas         │         │
│  │ • Multi-agent   │    │ • Signals        │         │
│  │ • Git storage   │    │ • Checkpoints    │         │
│  └─────────────────┘    └─────────────────┘         │
│           │                      │                   │
│           ▼                      ▼                   │
│  ┌─────────────────────────────────────────────┐│
│  │        COORDINATION BRIDGE            ││
│  │                                           ││
│  │ • Issue ↔ Workflow mapping             ││
│  │ • Agent assignment                   ││
│  │ • Result storage                    ││
│  │ • Performance analytics              ││
│  │ • Signal routing                   ││
│  └─────────────────────────────────────────────┘│
│           │                                           │
│           ▼                                           │
│  ┌─────────────────────────────────────────────┐│
│  │            AGENT LAYER                   ││
│  │                                           ││
│  │ • ResearchAgent      ← Activity       ││
│  │ • POCAgent         ← Saga          ││
│  │ • DocumentationAgent ← Activity       ││
│  │ • ValidationAgent   ← Activity       ││
│  └─────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

## 🔧 Component Responsibilities

### 1. Beads Core Layer

**Purpose**: Issue tracking and multi-agent coordination
**Key Responsibilities**:
- Issue lifecycle management (create, update, close)
- Dependency tracking and ready work calculation
- Multi-agent work assignment and handoffs
- Git-backed persistence and synchronization
- Formula and molecule workflow templates

**Data Flow**:
```
User Commands → Beads CLI → SQLite (beads.db) → JSONL (issues.jsonl) → Git Repository
```

### 2. Tempolite Core Layer  

**Purpose**: Durable workflow execution engine
**Key Responsibilities**:
- Activity execution with result capture
- Saga coordination for transactional operations
- Signal-based workflow communication
- Checkpoint creation and recovery
- Performance monitoring and metrics

**Data Flow**:
```
Workflow Definition → Tempolite Engine → Activities/Sagas → SQLite (workflow.db) → Checkpoints
```

### 3. Coordination Bridge

**Purpose**: Bridge beads and tempolite for seamless integration
**Key Responsibilities**:
- Map beads issues to tempolite workflows
- Coordinate agent handoffs between systems
- Store execution results and metadata
- Provide unified API for workflow management
- Handle system recovery and consistency

**Bridge Schema**:
```sql
-- Issue ↔ Workflow mapping
CREATE TABLE workflow_mappings (
    beads_issue_id TEXT PRIMARY KEY,
    tempolite_workflow_id TEXT NOT NULL,
    workflow_type TEXT NOT NULL, -- research, poc, documentation, validation
    status TEXT DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Agent assignments and workload
CREATE TABLE agent_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    status TEXT DEFAULT 'assigned',
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Execution results storage
CREATE TABLE workflow_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    result_type TEXT NOT NULL, -- findings, poc_results, documentation, validation
    result_data JSON,
    confidence_score REAL,
    execution_time_ms INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);
```

## 🔄 Data Flow Architecture

### Workflow Initiation Flow

```
User executes "bd workflow start research ..."
         ↓
    1. Beads creates issue bd-xyz
         ↓
    2. Coordination bridge maps issue → workflow wf-001
         ↓
    3. Tempolite starts workflow wf-001 with context
         ↓
    4. ResearchAgent activities execute
         ↓
    5. Results stored in coordination bridge
         ↓
    6. Beads issue updated with findings
```

### Agent Handoff Flow

```
ResearchAgent completes workflow wf-001.1
         ↓
    1. Results stored in workflow_results table
         ↓
    2. Bridge creates handoff signal in tempolite
         ↓
    3. Beads issue bd-xyz updated with handoff comment
         ↓
    4. POCAgent assigned to next workflow step wf-001.2
         ↓
    5. Tempolite starts POC saga with research context
```

### Recovery Flow

```
System crash/restart detected
         ↓
    1. Bridge checks incomplete workflows in database
         ↓
    2. Tempolite restores from last checkpoint
         ↓
    3. Beads verifies issue state consistency
         ↓
    4. Bridge resumes workflow execution
         ↓
    5. Active agents reconnect to workflows
```

## 🎯 Design Patterns

### 1. Bridge Pattern

**Problem**: Coordinate between two independent systems (Beads + Tempolite)
**Solution**: Coordination bridge with mapping tables and unified API

```go
type CoordinationBridge struct {
    beadsClient    *BeadsClient
    tempolite      *tempolite.Tempolite
    db             *sql.DB  // Coordination database
    eventBus       *EventBus
}

func (cb *CoordinationBridge) StartWorkflow(issueID string, workflowType string) error {
    // 1. Create beads issue
    issue, err := cb.beadsClient.CreateIssue(/*...*/)
    if err != nil {
        return err
    }
    
    // 2. Create mapping
    workflowID := generateWorkflowID()
    _, err = cb.db.Exec(`
        INSERT INTO workflow_mappings (beads_issue_id, tempolite_workflow_id, workflow_type)
        VALUES (?, ?, ?)
    `, issue.ID, workflowID, workflowType)
    
    // 3. Start tempolite workflow
    return cb.tempolite.StartWorkflow(workflowID, /*...*/)
}
```

### 2. Activity-to-Agent Pattern

**Problem**: Map generic activities to specialized agents
**Solution**: Activity registry with agent-specific implementations

```go
type ActivityRegistry map[string]ActivityFactory

type ActivityFactory func(context ActivityContext) Activity

func (ar ActivityRegistry) Register(agentType string, factory ActivityFactory) {
    ar[agentType] = factory
}

// Usage
registry := make(ActivityRegistry)
registry.Register("research", NewResearchActivity)
registry.Register("poc", NewPOCActivity)
registry.Register("documentation", NewDocumentationActivity)
registry.Register("validation", NewValidationActivity)
```

### 3. Saga Compensation Pattern

**Problem**: Ensure data consistency during complex operations
**Solution**: Tempolite sagas with compensation actions

```go
func POCImplementation(ctx tempolite.WorkflowContext, researchID string) error {
    saga := tempolite.NewSaga().
        Add(func(tc tempolite.TransactionContext) error {
            // Create implementation
            return CreateImplementation(tc, researchID)
        }, func(cc tempolite.CompensationContext) error {
            // Compensate: clean up implementation
            return CleanupImplementation(cc, researchID)
        }).
        Add(func(tc tempolite.TransactionContext) error {
            // Run tests
            return RunTests(tc, researchID)
        }, func(cc tempolite.CompensationContext) error {
            // Compensate: clean up test artifacts
            return CleanupTestArtifacts(cc, researchID)
        }).
        Build()
    
    return ctx.Saga("poc_implementation", saga).Get(nil)
}
```

### 4. Signal Coordination Pattern

**Problem**: Coordinate async events between workflows
**Solution**: Tempolite signals with bridge routing

```go
func (cb *CoordinationBridge) CoordinateHandoff(workflowID string, fromAgent, toAgent string) error {
    // 1. Create handoff signal
    signal := map[string]interface{}{
        "from_agent": fromAgent,
        "to_agent":   toAgent,
        "timestamp":  time.Now(),
    }
    
    // 2. Send signal through tempolite
    err := cb.tempolite.Signal(workflowID, "agent_handoff", signal)
    if err != nil {
        return err
    }
    
    // 3. Update beads with handoff comment
    return cb.beadsClient.AddComment(workflowID, 
        fmt.Sprintf("Handoff from %s to %s", fromAgent, toAgent))
}
```

## 🗄️ Database Schema Design

### Coordination Database (coordination.db)

```sql
-- Core mapping table
CREATE TABLE workflow_mappings (
    beads_issue_id TEXT PRIMARY KEY,
    tempolite_workflow_id TEXT NOT NULL UNIQUE,
    workflow_type TEXT NOT NULL CHECK (workflow_type IN ('research', 'poc', 'documentation', 'validation')),
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'completed', 'failed', 'paused')),
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Agent workload tracking
CREATE TABLE agent_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    step_number INTEGER DEFAULT 1,
    assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    status TEXT DEFAULT 'assigned' CHECK (status IN ('assigned', 'started', 'completed', 'failed')),
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Execution results with rich metadata
CREATE TABLE workflow_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    result_type TEXT NOT NULL,
    result_data JSON,
    confidence_score REAL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    execution_time_ms INTEGER,
    resource_usage JSON,
    artifacts JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Performance metrics
CREATE TABLE workflow_performance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    step_name TEXT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    duration_ms INTEGER,
    success BOOLEAN DEFAULT FALSE,
    error_message TEXT,
    memory_usage_mb INTEGER,
    cpu_usage_percent REAL,
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);
```

### Indexes for Performance

```sql
-- Workflow lookup indexes
CREATE INDEX idx_workflow_mappings_status ON workflow_mappings(status);
CREATE INDEX idx_workflow_mappings_type ON workflow_mappings(workflow_type);
CREATE INDEX idx_workflow_mappings_created ON workflow_mappings(created_at);

-- Agent assignment indexes
CREATE INDEX idx_agent_assignments_agent ON agent_assignments(agent_id, status);
CREATE INDEX idx_agent_assignments_workflow ON agent_assignments(workflow_id, step_number);

-- Result lookup indexes
CREATE INDEX idx_workflow_results_workflow ON workflow_results(workflow_id);
CREATE INDEX idx_workflow_results_agent_type ON workflow_results(agent_type, result_type);

-- Performance analytics indexes
CREATE INDEX idx_workflow_performance_workflow ON workflow_performance(workflow_id);
CREATE INDEX idx_workflow_performance_duration ON workflow_performance(duration_ms DESC);
```

## 🚀 Performance Considerations

### Connection Management
```go
type DatabaseManager struct {
    beadsDB    *sql.DB  // Single writer for beads
    coordDB     *sql.DB  // Single writer for coordination
    tempoliteDB *sql.DB  // Single writer for tempolite
    readDBs     []*sql.DB // Multiple readers for queries
}
```

### Caching Strategy
```go
type CacheManager struct {
    workflowCache   map[string]*WorkflowMapping
    agentCache      map[string]*AgentAssignment
    resultsCache    map[string][]*WorkflowResult
    cacheTTL       time.Duration
    mu             sync.RWMutex
}
```

### Batch Operations
```go
func (cb *CoordinationBridge) BatchUpdateResults(workflowID string, results []*WorkflowResult) error {
    tx, err := cb.coordDB.Begin()
    if err != nil {
        return err
    }
    defer tx.Rollback()
    
    stmt, err := tx.Prepare(`
        INSERT INTO workflow_results (workflow_id, agent_type, result_type, result_data, confidence_score, execution_time_ms)
        VALUES (?, ?, ?, ?, ?, ?)
    `)
    if err != nil {
        return err
    }
    defer stmt.Close()
    
    for _, result := range results {
        _, err := stmt.Exec(
            workflowID, result.AgentType, result.ResultType,
            result.Data, result.Confidence, result.ExecutionTime,
        )
        if err != nil {
            return err
        }
    }
    
    return tx.Commit()
}
```

## 🛡️ Error Handling & Recovery

### Failure Modes and Recovery Strategies

| Failure Mode | Detection | Recovery Strategy |
|---------------|------------|------------------|
| Beads database corruption | Integrity check failure | Rebuild from JSONL |
| Tempolite workflow crash | Heartbeat timeout | Restore from checkpoint |
| Bridge database inconsistency | Cross-system validation | Reconstruct from source systems |
| Network partition (Git sync) | Git operation timeout | Queue changes, retry when available |
| Agent process crash | No heartbeat signal | Reassign work to other agents |

### Recovery Implementation

```go
func (cb *CoordinationBridge) RecoverFromCrash() error {
    // 1. Check system health
    if err := cb.healthCheck(); err != nil {
        return fmt.Errorf("system health check failed: %w", err)
    }
    
    // 2. Restore tempolite workflows
    incompleteWorkflows, err := cb.getIncompleteWorkflows()
    if err != nil {
        return err
    }
    
    for _, workflow := range incompleteWorkflows {
        if err := cb.tempolite.RestoreWorkflow(workflow.ID); err != nil {
            log.Printf("Failed to restore workflow %s: %v", workflow.ID, err)
            continue
        }
    }
    
    // 3. Reconcile beads state
    if err := cb.reconcileBeadsState(); err != nil {
        return err
    }
    
    // 4. Resume agent assignments
    return cb.resumeAgentAssignments()
}
```

## 🔗 Cross-References

- [Database Schema](./02-database-schema.md) - Complete database design
- [API Design](./03-api-design.md) - REST API and internal interfaces
- [Security Model](./04-security.md) - Security and authentication
- [Deployment Architecture](./05-deployment.md) - Production deployment patterns
- [Migration Strategy](./06-migration.md) - Migration from existing systems

---

**Next**: Read the [Database Schema](./02-database-schema.md) for detailed database design.