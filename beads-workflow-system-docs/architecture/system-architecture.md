# System Architecture Overview

## 1. Executive Summary

The Beads-Workflow-System is a hybrid workflow automation platform that combines the multi-agent coordination capabilities of Beads with the durable execution engine of Tempolite. This architecture enables persistent, coordinated workflow automation for AI agents with Git-backed persistence and SQLite-based durability.

## 2. Architecture Philosophy

### 2.1 Design Principles

**Separation of Concerns**: Clear boundaries between coordination (Beads) and execution (Tempolite) allow each system to evolve independently while maintaining integration through a well-defined bridge layer.

**Event-Driven Communication**: All system interactions occur through events, enabling loose coupling and independent scaling of components.

**Git-Native Persistence**: All coordination state is stored in Git through Beads' JSONL format, providing automatic versioning, audit trails, and conflict resolution.

**SQLite-Based Execution**: Workflow execution state is stored in SQLite with Write-Ahead Logging (WAL) mode for high-performance, reliable local-first operation.

**Local-First Architecture**: All operations work offline with optional synchronization, ensuring reliability in disconnected environments.

## 3. System Components

### 3.1 Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CLIENT LAYER                                     │
│                                                                              │
│   ┌──────────────┐     ┌──────────────┐     ┌──────────────┐              │
│   │   CLI Tool   │     │ REST API     │     │ WebSocket    │              │
│   │   (cobra)    │     │ (gin)        │     │ (ws)         │              │
│   └──────┬───────┘     └──────┬───────┘     └──────┬───────┘              │
│          └─────────────────────┴─────────────────────┘                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ HTTP/WS
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         API SERVER LAYER                                   │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                    REST API Handlers                               │  │
│   │                                                                      │  │
│   │  POST   /api/v1/workflows          - Create workflow               │  │
│   │  GET    /api/v1/workflows/:id      - Get workflow status           │  │
│   │  GET    /api/v1/workflows          - List workflows                │  │
│   │  DELETE /api/v1/workflows/:id      - Cancel workflow               │  │
│   │  GET    /api/v1/workflows/:id/logs - Get workflow logs             │  │
│   │                                                                      │  │
│   │  POST   /api/v1/agents             - Register agent                │  │
│   │  GET    /api/v1/agents/:id/status  - Get agent status              │  │
│   │  POST   /api/v1/workflows/:id/handoff - Handoff workflow          │  │
│   │                                                                      │  │
│   │  GET    /api/v1/analytics/performance - Performance metrics        │  │
│   │                                                                      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │               Authentication & Authorization                         │  │
│   │  JWT tokens, API keys, rate limiting, CORS                           │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   │ Internal API calls
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                     COORDINATION BRIDGE LAYER                              │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │           Beads-Workflow Bridge                                     │  │
│   │                                                                      │  │
│   │   Responsibilities:                                                  │  │
│   │   • Map beads issues to tempolite workflows                         │  │
│   │   • Coordinate agent assignments                                     │  │
│   │   • Store execution results                                          │  │
│   │   • Handle agent handoffs                                            │  │
│   │   • Provide unified API for workflow management                     │  │
│   │   • Maintain cross-system consistency                               │  │
│   │                                                                      │  │
│   │   Components:                                                        │  │
│   │   ├─ WorkflowManager     - Workflow lifecycle management           │  │
│   │   ├─ AgentCoordinator    - Agent assignment and handoffs           │  │
│   │   ├─ ResultsStorage      - Execution results storage               │  │
│   │   ├─ EventBus            - Event routing and distribution          │  │
│   │   └─ StateReconciler     - Cross-system state synchronization      │  │
│   │                                                                      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
│                                    │                                        │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │           Database Manager                                          │  │
│   │                                                                      │  │
│   │   Manages three separate databases:                                  │  │
│   │   • Beads DB (beads.db)        - Issue tracking                     │  │
│   │   • Tempolite DB (tempolite.db) - Workflow execution                │  │
│   │   • Coordination DB (coord.db)  - Bridge layer                      │  │
│   │                                                                      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
                                   │ Internal APIs
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      BACKEND SERVICES LAYER                                │
│                                                                              │
│   ┌─────────────────────────┐    ┌───────────────────────────────────────┐ │
│   │       BEADS CORE        │    │         TEMPOLITE CORE                │ │
│   │                         │    │                                       │ │
│   │   • Issue Management   │    │   • Activity Execution                │ │
│   │   • Dependencies        │    │   • Saga Coordination                 │ │
│   │   • Multi-Agent         │    │   • Signal Handling                   │ │
│   │   • Git Sync            │    │   • Checkpoint & Recovery             │ │
│   │   • Formula System      │    │   • Workflow Versioning               │ │
│   │                         │    │                                       │ │
│   │   Storage:              │    │   Storage:                            │ │
│   │   • issues.jsonl       │    │   • SQLite with WAL mode              │ │
│   │   • .beads/ directory   │    │   • Automated checkpoints             │ │
│   │   • Git repository      │    │   • Event sourcing                    │ │
│   └─────────────────────────┘    └───────────────────────────────────────┘ │
│                                                                              │
│   ┌─────────────────────────────────────────────────────────────────────┐  │
│   │                     AGENT IMPLEMENTATIONS                           │  │
│   │                                                                      │  │
│   │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐ │  │
│   │   │  Research   │  │     POC     │  │  Document   │  │ Validation│ │  │
│   │   │   Agent     │  │   Agent     │  │   Agent     │  │   Agent   │ │  │
│   │   │             │  │             │  │             │  │           │ │  │
│   │   │ Library     │  │ Implementation│  │ Documentation│  │ Automated │ │  │
│   │   │ Discovery   │  │ Build       │  │ Generation  │  │  Tests    │ │  │
│   │   │ Analysis    │  │ Test        │  │ Review      │  │ Review    │ │  │
│   │   │ Synthesis   │  │ Benchmark   │  │ Update      │  │ Report    │ │  │
│   │   └─────────────┘  └─────────────┘  └─────────────┘  └───────────┘ │  │
│   │                                                                      │  │
│   └─────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Component Responsibilities

#### 3.2.1 CLI Tool (cmd/workflow)
**Location**: `cmd/workflow/`

**Responsibilities**:
- Parse and validate user commands
- Transform user input to API calls
- Display formatted output (table, JSON, YAML)
- Support interactive mode with prompts
- Handle authentication and configuration

**Key Components**:
```go
type CLIRoot struct {
    commands    map[string]*cobra.Command
    apiClient   *APIClient
    output      OutputFormatter
    config      *Config
}
```

**Command Structure**:
```
workflow start <type> <title> [flags]
workflow status <id>
workflow list [filters]
workflow cancel <id>
workflow results <id>
workflow logs <id>
agent register <config>
agent status <id>
agent list
analytics performance [period]
```

#### 3.2.2 REST API Server (internal/api)
**Location**: `internal/api/`

**Responsibilities**:
- HTTP request routing and handling
- Request validation and authentication
- Response formatting and caching
- Rate limiting and security
- WebSocket management for real-time updates

**Key Components**:
```go
type APIServer struct {
    router      *gin.Engine
    bridge      *CoordinationBridge
    auth        *AuthMiddleware
    rateLimiter *RateLimiter
    logger      *zap.Logger
}
```

**API Endpoints**:
```go
// Workflow endpoints
POST   /api/v1/workflows          - Start workflow
GET    /api/v1/workflows/:id      - Get workflow status
GET    /api/v1/workflows          - List workflows
PUT    /api/v1/workflows/:id      - Update workflow
DELETE /api/v1/workflows/:id      - Cancel workflow
GET    /api/v1/workflows/:id/results - Get results
GET    /api/v1/workflows/:id/logs    - Get logs

// Agent endpoints
POST   /api/v1/agents             - Register agent
GET    /api/v1/agents             - List agents
GET    /api/v1/agents/:id         - Get agent
GET    /api/v1/agents/:id/status  - Get agent status
PUT    /api/v1/agents/:id/config  - Update config
DELETE /api/v1/agents/:id         - Unregister agent

// Analytics endpoints
GET    /api/v1/analytics/performance - Performance metrics
GET    /api/v1/analytics/workflows    - Workflow analytics
GET    /api/v1/analytics/agents       - Agent analytics
GET    /api/v1/analytics/trends       - Trend analysis
```

#### 3.2.3 Coordination Bridge (internal/bridge)
**Location**: `internal/bridge/`

**Responsibilities**:
- Map beads issues to tempolite workflows
- Coordinate agent assignments and handoffs
- Store and retrieve execution results
- Maintain cross-system consistency
- Handle system recovery and state reconciliation

**Key Components**:
```go
type CoordinationBridge struct {
    beadsClient    *beads.Client
    tempolite      *tempolite.Tempolite
    dbManager      *database.DatabaseManager
    eventBus       *EventBus
    workflowMgr    *WorkflowManager
    agentCoord     *AgentCoordinator
    resultsStorage *ResultsStorage
    stateReconciler *StateReconciler
    cache          *CacheManager
}
```

**Core Operations**:
```go
// Workflow lifecycle
func (cb *CoordinationBridge) StartWorkflow(ctx context.Context, req *StartWorkflowRequest) (*Workflow, error)
func (cb *CoordinationBridge) GetWorkflow(ctx context.Context, id string) (*Workflow, error)
func (cb *CoordinationBridge) UpdateWorkflowStatus(ctx context.Context, id, status string) error
func (cb *CoordinationBridge) CancelWorkflow(ctx context.Context, id, reason string) error

// Agent management
func (cb *CoordinationBridge) RegisterAgent(ctx context.Context, agent *Agent) error
func (cb *CoordinationBridge) AssignAgent(ctx context.Context, workflowID, agentID string) error
func (cb *CoordinationBridge) CoordinateHandoff(ctx context.Context, workflowID, fromAgent, toAgent string) error

// Results storage
func (cb *CoordinationBridge) StoreResults(ctx context.Context, workflowID string, results *Results) error
func (cb *CoordinationBridge) GetResults(ctx context.Context, workflowID string) ([]*Results, error)

// Analytics
func (cb *CoordinationBridge) GetAnalytics(ctx context.Context, filters *AnalyticsFilters) (*Analytics, error)
```

#### 3.2.4 Database Manager (internal/database)
**Location**: `internal/database/`

**Responsibilities**:
- Initialize and configure databases
- Manage database connections and connection pooling
- Apply SQLite optimizations and pragmas
- Handle schema creation and migrations
- Ensure transactional consistency

**Key Components**:
```go
type DatabaseManager struct {
    beadsDB       *sql.DB  // Beads database (read-only access)
    coordDB       *sql.DB  // Coordination database (read-write)
    tempoliteDB   *sql.DB  // Tempolite database (read-only access)
    
    config        *DatabaseConfig
    logger        *zap.Logger
    mu            sync.RWMutex
}
```

**Database Configuration**:
```go
type DatabaseConfig struct {
    CoordinationDB struct {
        Path            string
        MaxOpenConns    int
        MaxIdleConns    int
        ConnMaxLifetime time.Duration
        BusyTimeout     time.Duration
    }
}
```

**SQLite Optimizations**:
```sql
PRAGMA journal_mode = WAL;           -- Write-Ahead Logging
PRAGMA synchronous = NORMAL;         -- Balanced safety/performance
PRAGMA cache_size = 10000;          -- 10MB cache
PRAGMA temp_store = MEMORY;         -- In-memory temp tables
PRAGMA mmap_size = 268435456;       -- 256MB memory mapping
PRAGMA wal_autocheckpoint = 1000;   -- Auto-checkpoint
PRAGMA foreign_keys = ON;            -- Enforce FK constraints
```

#### 3.2.5 Agent Framework (internal/agents)
**Location**: `internal/agents/`

**Responsibilities**:
- Implement specialized agents (research, POC, documentation, validation)
- Define agent interface and lifecycle
- Handle agent registration and workload management
- Execute workflow activities
- Report progress and results

**Agent Interface**:
```go
type Agent interface {
    // Lifecycle
    Initialize(ctx context.Context, config *AgentConfig) error
    Start(ctx context.Context) error
    Stop(ctx context.Context) error
    Shutdown(ctx context.Context) error
    
    // Capabilities
    GetType() string
    GetCapabilities() []string
    GetMaxWorkload() int
    GetCurrentWorkload() int
    
    // Task execution
    AssignTask(ctx context.Context, task *Task) error
    ExecuteActivity(ctx context.Context, activity string, params interface{}) (interface{}, error)
    CompleteTask(ctx context.Context, taskID string, results *Results) error
    FailTask(ctx context.Context, taskID string, err error) error
    
    // Health monitoring
    HealthCheck(ctx context.Context) (*HealthStatus, error)
    GetMetrics(ctx context.Context) (*AgentMetrics, error)
}
```

#### 3.2.6 Beads Integration (pkg/beads)
**Location**: `pkg/beads/`

**Responsibilities**:
- Interface with Beads issue tracking system
- Create and update issues
- Manage dependencies and ready work
- Handle multi-agent coordination
- Sync with Git repository

**Key Components**:
```go
type BeadsClient struct {
    repoPath    string
    db          *sql.DB
    gitClient   *GitClient
}

type Issue struct {
    ID          string
    Title       string
    Description string
    Type        string
    Status      string
    Priority    int
    Assignee    string
    Labels      []string
    CreatedAt   time.Time
    UpdatedAt   time.Time
}
```

#### 3.2.7 Tempolite Integration (pkg/tempolite)
**Location**: `pkg/tempolite/`

**Responsibilities**:
- Interface with Tempolite workflow engine
- Start and manage workflow execution
- Handle activities, sagas, and signals
- Manage checkpoints and recovery
- Provide workflow metrics

**Key Components**:
```go
type TempoliteClient struct {
    db          *sql.DB
    engine      *WorkflowEngine
    registry    *ActivityRegistry
    checkpointMgr *CheckpointManager
}

type WorkflowEngine struct {
    ctx         context.Context
    workflows   map[string]*WorkflowInstance
    signals     map[string]chan interface{}
    mu          sync.RWMutex
}
```

## 4. Data Flow

### 4.1 Workflow Initiation Flow

```
User executes CLI command
         │
         ▼
┌──────────────────────┐
│  CLI parses command  │
│  validates inputs   │
│  loads config        │
└──────────┬───────────┘
           │
           │ HTTP POST /api/v1/workflows
           ▼
┌──────────────────────┐
│  API server receives │
│  request, validates │
│  authentication      │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│  CoordinationBridge  │
│  StartWorkflow()    │
└──────────┬───────────┘
           │
           │ 1. Create beads issue
           ▼
┌──────────────────────┐    ┌──────────────────┐
│  BeadsClient         │───►│  beads.db        │
│  CreateIssue()      │    │  issues.jsonl    │
└──────────┬───────────┘    └──────────────────┘
           │
           │ 2. Create workflow mapping
           ▼
┌──────────────────────┐    ┌──────────────────┐
│  DatabaseManager     │───►│  coordination.db │
│  Insert mapping     │    │  workflow_mappings│
└──────────┬───────────┘    └──────────────────┘
           │
           │ 3. Start tempolite workflow
           ▼
┌──────────────────────┐    ┌──────────────────┐
│  TempoliteClient     │───►│  tempolite.db    │
│  StartWorkflow()    │    │  execution_log   │
└──────────┬───────────┘    └──────────────────┘
           │
           │ 4. Assign agent
           ▼
┌──────────────────────┐    ┌──────────────────┐
│  AgentCoordinator    │───►│  Agent           │
│  AssignAgent()      │    │  implementation  │
└──────────────────────┘    └──────────────────┘
```

**Transaction Boundary**: Steps 1-4 occur within a single database transaction on the coordination database to ensure atomicity. If any step fails, all changes are rolled back.

**Code Example**:
```go
func (cb *CoordinationBridge) StartWorkflow(ctx context.Context, req *StartWorkflowRequest) (*Workflow, error) {
    // Start transaction
    tx, err := cb.dbManager.GetCoordinationTx(ctx)
    if err != nil {
        return nil, err
    }
    defer tx.Rollback()
    
    // Step 1: Create beads issue
    issue, err := cb.beadsClient.CreateIssue(ctx, &beads.CreateIssueRequest{
        Title:    req.IssueTitle,
        Type:     "task",
        Priority: req.Priority,
    })
    if err != nil {
        return nil, fmt.Errorf("failed to create beads issue: %w", err)
    }
    
    // Step 2: Create workflow mapping
    workflowID := generateWorkflowID(req.WorkflowType)
    metadata, _ := json.Marshal(req.Variables)
    
    _, err = tx.ExecContext(ctx, `
        INSERT INTO workflow_mappings 
        (beads_issue_id, tempolite_workflow_id, workflow_type, priority, metadata) 
        VALUES (?, ?, ?, ?, ?)
    `, issue.ID, workflowID, req.WorkflowType, req.Priority, metadata)
    if err != nil {
        return nil, fmt.Errorf("failed to create workflow mapping: %w", err)
    }
    
    // Step 3: Start tempolite workflow
    workflowDef := cb.buildWorkflowDefinition(req)
    err = cb.tempolite.StartWorkflow(ctx, workflowID, workflowDef)
    if err != nil {
        return nil, fmt.Errorf("failed to start tempolite workflow: %w", err)
    }
    
    // Step 4: Assign agent
    _, err = tx.ExecContext(ctx, `
        INSERT INTO agent_assignments 
        (workflow_id, agent_type, agent_id, step_number, status) 
        VALUES (?, ?, ?, ?, ?)
    `, workflowID, req.AgentType, cb.generateAgentID(req.AgentType), 1, "assigned")
    if err != nil {
        return nil, fmt.Errorf("failed to assign agent: %w", err)
    }
    
    // Commit transaction
    if err := tx.Commit(); err != nil {
        return nil, fmt.Errorf("failed to commit transaction: %w", err)
    }
    
    // Emit event
    cb.eventBus.Publish(EventWorkflowStarted, &EventWorkflowStarted{
        WorkflowID:     workflowID,
        BeadsIssueID:   issue.ID,
        WorkflowType:   req.WorkflowType,
    })
    
    return &Workflow{
        ID:             workflowID,
        BeadsIssueID:   issue.ID,
        Type:           req.WorkflowType,
        Status:         "active",
        Priority:       req.Priority,
        StartedAt:      time.Now(),
    }, nil
}
```

### 4.2 Agent Handoff Flow

```
Agent A completes workflow step
         │
         ▼
┌──────────────────────┐
│  Activity completes  │
│  results available  │
└──────────┬───────────┘
           │
           │ Event: step_completed
           ▼
┌──────────────────────┐
│  CoordinationBridge  │
│  handleStepCompleted()│
└──────────┬───────────┘
           │
           │ 1. Store results
           ▼
┌──────────────────────┐
│  ResultsStorage      │
│  StoreResults()     │
└──────────┬───────────┘
           │
           │ 2. Update beads issue
           ▼
┌──────────────────────┐
│  BeadsClient         │
│  AddComment()       │
└──────────┬───────────┘
           │
           │ 3. Create handoff signal
           ▼
┌──────────────────────┐
│  TempoliteClient     │
│  Signal()           │
└──────────┬───────────┘
           │
           │ 4. Update agent assignment
           ▼
┌──────────────────────┐
│  AgentCoordinator    │
│  UpdateAssignment() │
└──────────┬───────────┘
           │
           │ 5. Assign new agent
           ▼
┌──────────────────────┐
│  Agent B receives    │
│  signal and starts   │
└──────────────────────┘
```

**Code Example**:
```go
func (cb *CoordinationBridge) CoordinateHandoff(ctx context.Context, workflowID, fromAgent, toAgent string) error {
    tx, err := cb.dbManager.GetCoordinationTx(ctx)
    if err != nil {
        return err
    }
    defer tx.Rollback()
    
    // 1. Get workflow mapping
    mapping, err := cb.getWorkflowMappingTx(ctx, tx, workflowID)
    if err != nil {
        return err
    }
    
    // 2. Create handoff signal in tempolite
    signal := map[string]interface{}{
        "from_agent": fromAgent,
        "to_agent":   toAgent,
        "timestamp":  time.Now(),
        "mapping_id": mapping.BeadsIssueID,
    }
    
    if err := cb.tempolite.Signal(ctx, workflowID, "agent_handoff", signal); err != nil {
        return fmt.Errorf("failed to send handoff signal: %w", err)
    }
    
    // 3. Update beads issue with handoff comment
    comment := fmt.Sprintf("🔄 Handoff from %s to %s\n\n%s completed their work. Passing to %s for next phase.",
        fromAgent, toAgent, fromAgent, toAgent)
    
    if err := cb.beadsClient.AddComment(ctx, mapping.BeadsIssueID, comment); err != nil {
        cb.logger.Warn("Failed to add handoff comment", zap.Error(err))
    }
    
    // 4. Update current assignment
    _, err = tx.ExecContext(ctx, `
        UPDATE agent_assignments 
        SET status = 'completed', completed_at = ?
        WHERE workflow_id = ? AND agent_id = ?
    `, time.Now(), workflowID, fromAgent)
    if err != nil {
        return fmt.Errorf("failed to complete current assignment: %w", err)
    }
    
    // 5. Create new assignment
    nextStep := mapping.CurrentStep + 1
    _, err = tx.ExecContext(ctx, `
        INSERT INTO agent_assignments 
        (workflow_id, agent_id, agent_type, step_number, handoff_from, status) 
        VALUES (?, ?, ?, ?, ?, ?)
    `, workflowID, toAgent, cb.getAgentTypeForStep(nextStep), nextStep, fromAgent, "assigned")
    if err != nil {
        return fmt.Errorf("failed to create new assignment: %w", err)
    }
    
    if err := tx.Commit(); err != nil {
        return fmt.Errorf("failed to commit handoff: %w", err)
    }
    
    // Emit event
    cb.eventBus.Publish(EventAgentHandoff, &EventAgentHandoff{
        WorkflowID: workflowID,
        FromAgent:  fromAgent,
        ToAgent:    toAgent,
        StepNumber: nextStep,
    })
    
    return nil
}
```

## 5. Database Architecture

### 5.1 Database Separation Strategy

The system uses **three separate SQLite databases** for architectural isolation:

| Database | Purpose | Managed By | Access Pattern | Criticality |
|----------|---------|------------|----------------|-------------|
| **beads.db** | Issue tracking and coordination | Beads | Read-mostly, periodic sync | High |
| **tempolite.db** | Workflow execution state | Tempolite | Heavy write, transaction-heavy | Critical |
| **coordination.db** | Bridge layer mapping and results | Workflow system | Balanced read/write | Critical |

**Why Three Databases?**

1. **Isolation**: Prevents schema conflicts and allows independent evolution
2. **Performance**: Each database can be optimized independently
3. **Recovery**: Individual database recovery without affecting others
4. **Maintenance**: Independent backup and migration strategies
5. **Clear Ownership**: Beads manages beads.db, Tempolite manages tempolite.db

### 5.2 Coordination Database Schema

See `database/coordination-schema.md` for complete schema.

### 5.3 Database Connection Management

**Single Writer Pattern**: Each database has exactly one writer connection to prevent SQLite locking issues.

```go
type DatabaseManager struct {
    // Single writer for coordination database
    coordWriter   *sql.DB
    
    // Multiple readers for queries
    coordReaders  []*sql.DB
    readerIndex   uint64
    
    // Configuration
    config        *DatabaseConfig
    logger        *zap.Logger
}

func (dm *DatabaseManager) GetCoordinationWriter() (*sql.DB, error) {
    return dm.coordWriter, nil
}

func (dm *DatabaseManager) GetCoordinationReader() (*sql.DB, error) {
    // Round-robin selection of reader
    index := atomic.AddUint64(&dm.readerIndex, 1) % uint64(len(dm.coordReaders))
    return dm.coordReaders[index], nil
}

func (dm *DatabaseManager) ExecuteReadQuery(ctx context.Context, query string, args ...interface{}) (*sql.Rows, error) {
    db, err := dm.GetCoordinationReader()
    if err != nil {
        return nil, err
    }
    return db.QueryContext(ctx, query, args...)
}

func (dm *DatabaseManager) ExecuteWriteQuery(ctx context.Context, query string, args ...interface{}) (sql.Result, error) {
    db, err := dm.GetCoordinationWriter()
    if err != nil {
        return nil, err
    }
    return db.ExecContext(ctx, query, args...)
}
```

## 6. Error Handling Strategy

### 6.1 Error Hierarchy

```go
// Base error types
type WorkflowError struct {
    Code    string
    Message string
    Cause   error
}

func (e *WorkflowError) Error() string {
    if e.Cause != nil {
        return fmt.Sprintf("%s: %s (caused by: %v)", e.Code, e.Message, e.Cause)
    }
    return fmt.Sprintf("%s: %s", e.Code, e.Message)
}

func (e *WorkflowError) Unwrap() error {
    return e.Cause
}

// Specific error types
type ValidationError struct {
    WorkflowError
    Field   string
    Value   interface{}
}

type DatabaseError struct {
    WorkflowError
    Operation string
    Table     string
}

type AgentError struct {
    WorkflowError
    AgentID   string
    AgentType string
}

type ExternalServiceError struct {
    WorkflowError
    Service   string
    Endpoint  string
    StatusCode int
}
```

### 6.2 Error Recovery Patterns

**Retry with Exponential Backoff**:
```go
func RetryWithBackoff(operation func() error, maxRetries int) error {
    backoff := time.Second
    
    for attempt := 0; attempt < maxRetries; attempt++ {
        err := operation()
        if err == nil {
            return nil
        }
        
        // Check if error is retryable
        if !isRetryableError(err) {
            return err
        }
        
        if attempt < maxRetries-1 {
            time.Sleep(backoff)
            backoff *= 2
            if backoff > 30*time.Second {
                backoff = 30 * time.Second
            }
        }
    }
    
    return fmt.Errorf("operation failed after %d attempts", maxRetries)
}
```

**Circuit Breaker Pattern**:
```go
type CircuitBreaker struct {
    maxFailures     int
    failureCount    int
    lastFailureTime time.Time
    resetTimeout    time.Duration
    state          CircuitState
    mu             sync.RWMutex
}

type CircuitState int

const (
    CircuitClosed CircuitState = iota
    CircuitOpen
    CircuitHalfOpen
)

func (cb *CircuitBreaker) Call(operation func() error) error {
    cb.mu.Lock()
    
    if cb.state == CircuitOpen {
        if time.Since(cb.lastFailureTime) > cb.resetTimeout {
            cb.state = CircuitHalfOpen
            cb.failureCount = 0
        } else {
            cb.mu.Unlock()
            return fmt.Errorf("circuit breaker is open")
        }
    }
    
    cb.mu.Unlock()
    
    err := operation()
    
    cb.mu.Lock()
    defer cb.mu.Unlock()
    
    if err != nil {
        cb.failureCount++
        cb.lastFailureTime = time.Now()
        
        if cb.failureCount >= cb.maxFailures {
            cb.state = CircuitOpen
        }
        
        return err
    }
    
    // Success
    cb.failureCount = 0
    cb.state = CircuitClosed
    
    return nil
}
```

## 7. Performance Considerations

### 7.1 Caching Strategy

**Multi-Level Caching**:

```go
type CacheManager struct {
    // L1: In-memory cache (fast, volatile)
    l1Cache *ristretto.Cache
    
    // L2: Redis cache (distributed, persistent)
    l2Cache *redis.Client
    
    // L3: Database (slowest, most durable)
    db      *sql.DB
}

func (cm *CacheManager) GetWorkflow(ctx context.Context, workflowID string) (*Workflow, error) {
    // Try L1 cache
    if val, found := cm.l1Cache.Get(workflowID); found {
        return val.(*Workflow), nil
    }
    
    // Try L2 cache
    if cm.l2Cache != nil {
        val, err := cm.l2Cache.Get(ctx, workflowID).Result()
        if err == nil {
            var workflow Workflow
            if err := json.Unmarshal([]byte(val), &workflow); err == nil {
                // Populate L1 cache
                cm.l1Cache.Set(workflowID, &workflow, 1)
                return &workflow, nil
            }
        }
    }
    
    // Fallback to database
    workflow, err := cm.loadWorkflowFromDB(ctx, workflowID)
    if err != nil {
        return nil, err
    }
    
    // Populate caches
    cm.l1Cache.Set(workflowID, workflow, 1)
    if cm.l2Cache != nil {
        data, _ := json.Marshal(workflow)
        cm.l2Cache.Set(ctx, workflowID, data, 10*time.Minute)
    }
    
    return workflow, nil
}
```

### 7.2 Query Optimization

**Prepared Statements**:
```go
type PreparedQueries struct {
    getWorkflow        *sql.Stmt
    getAgentAssignments *sql.Stmt
    getResults         *sql.Stmt
    updateWorkflow     *sql.Stmt
}

func (dm *DatabaseManager) prepareQueries() (*PreparedQueries, error) {
    pq := &PreparedQueries{}
    
    var err error
    pq.getWorkflow, err = dm.coordWriter.Prepare(`
        SELECT beads_issue_id, workflow_type, status, priority, metadata
        FROM workflow_mappings 
        WHERE tempolite_workflow_id = ?
    `)
    if err != nil {
        return nil, err
    }
    
    pq.getAgentAssignments, err = dm.coordWriter.Prepare(`
        SELECT agent_id, agent_type, step_number, status, assigned_at
        FROM agent_assignments 
        WHERE workflow_id = ? 
        ORDER BY step_number
    `)
    if err != nil {
        return nil, err
    }
    
    // ... more prepared statements
    
    return pq, nil
}
```

**Covering Indexes**:
```sql
-- Include frequently accessed columns in index
CREATE INDEX idx_workflow_mappings_covering 
ON workflow_mappings(tempolite_workflow_id) 
INCLUDE (status, priority, created_at);

-- Multi-column index for common queries
CREATE INDEX idx_agent_assignments_workflow_status 
ON agent_assignments(workflow_id, status, step_number);
```

## 8. Security Architecture

### 8.1 Authentication

**JWT Token Authentication**:
```go
// internal/auth/jwt.go
package auth

type JWTManager struct {
    secretKey     []byte
    tokenExpiry   time.Duration
}

type Claims struct {
    jwt.RegisteredClaims
    UserID        string   `json:"user_id"`
    Roles         []string `json:"roles"`
    Permissions   []string `json:"permissions"`
}

func (jm *JWTManager) GenerateToken(userID string, roles, permissions []string) (string, error) {
    claims := Claims{
        RegisteredClaims: jwt.RegisteredClaims{
            ExpiresAt: jwt.NewNumericDate(time.Now().Add(jm.tokenExpiry)),
            IssuedAt:  jwt.NewNumericDate(time.Now()),
            NotBefore: jwt.NewNumericDate(time.Now()),
            Subject:   userID,
        },
        UserID:      userID,
        Roles:       roles,
        Permissions: permissions,
    }
    
    token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
    return token.SignedString(jm.secretKey)
}

func (jm *JWTManager) ValidateToken(tokenString string) (*Claims, error) {
    token, err := jwt.ParseWithClaims(tokenString, &Claims{}, func(token *jwt.Token) (interface{}, error) {
        return jm.secretKey, nil
    })
    
    if err != nil {
        return nil, err
    }
    
    claims, ok := token.Claims.(*Claims)
    if !ok || !token.Valid {
        return nil, fmt.Errorf("invalid token claims")
    }
    
    return claims, nil
}
```

### 8.2 Authorization

**Role-Based Access Control (RBAC)**:
```go
// internal/auth/rbac.go
package auth

type Permission string

const (
    WorkflowRead   Permission = "workflow:read"
    WorkflowWrite  Permission = "workflow:write"
    WorkflowDelete Permission = "workflow:delete"
    AgentRead      Permission = "agent:read"
    AgentWrite     Permission = "agent:write"
    AnalyticsRead  Permission = "analytics:read"
)

type Role struct {
    Name        string
    Permissions []Permission
}

var RoleDefinitions = map[string]Role{
    "admin": {
        Name: "admin",
        Permissions: []Permission{
            WorkflowRead, WorkflowWrite, WorkflowDelete,
            AgentRead, AgentWrite,
            AnalyticsRead,
        },
    },
    "operator": {
        Name: "operator",
        Permissions: []Permission{
            WorkflowRead, WorkflowWrite,
            AgentRead,
            AnalyticsRead,
        },
    },
    "viewer": {
        Name: "viewer",
        Permissions: []Permission{
            WorkflowRead,
            AgentRead,
            AnalyticsRead,
        },
    },
}

func HasPermission(userRoles []string, required Permission) bool {
    for _, roleName := range userRoles {
        role, exists := RoleDefinitions[roleName]
        if !exists {
            continue
        }
        
        for _, perm := range role.Permissions {
            if perm == required {
                return true
            }
        }
    }
    
    return false
}
```

## 9. Deployment Architecture

### 9.1 Docker Deployment

```dockerfile
# Multi-stage build
FROM golang:1.21-alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache git sqlite-dev gcc musl-dev

# Download dependencies
COPY go.mod go.sum ./
RUN go mod download

# Copy source code
COPY . .

# Build binaries
RUN CGO_ENABLED=1 GOOS=linux go build -a -ldflags '-extldflags "-static"' -o workflow-server ./cmd/server
RUN CGO_ENABLED=1 GOOS=linux go build -a -ldflags '-extldflags "-static"' -o workflow-cli ./cmd/workflow

# Production image
FROM alpine:3.18

# Install runtime dependencies
RUN apk add --no-cache ca-certificates sqlite-libs

WORKDIR /app

# Create directories
RUN mkdir -p /app/data/db /app/data/logs /app/configs

# Copy binaries
COPY --from=builder /app/workflow-server /usr/local/bin/
COPY --from=builder /app/workflow-cli /usr/local/bin/

# Copy default configs
COPY --from=builder /app/configs/ /app/configs/

# Create non-root user
RUN addgroup -g 1000 workflow && \
    adduser -u 1000 -G workflow -s /bin/sh -D workflow

# Set ownership
RUN chown -R workflow:workflow /app

USER workflow

# Environment variables
ENV WORKFLOW_CONFIG_PATH=/app/configs/production.yaml
ENV WORKFLOW_DATA_DIR=/app/data

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Expose ports
EXPOSE 8080 9090

# Start server
CMD ["workflow-server"]
```

### 9.2 Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: workflow-system
  labels:
    app: workflow-system
    version: v1.0.0
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: workflow-system
  template:
    metadata:
      labels:
        app: workflow-system
        version: v1.0.0
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      serviceAccountName: workflow-system
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      
      containers:
      - name: workflow-api
        image: ghcr.io/your-org/workflow-system:v1.0.0
        imagePullPolicy: Always
        
        ports:
        - name: http
          containerPort: 8080
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        
        env:
        - name: CONFIG_PATH
          value: "/etc/workflow/config.yaml"
        - name: DATA_DIR
          value: "/data"
        - name: LOG_LEVEL
          value: "info"
        
        volumeMounts:
        - name: config
          mountPath: /etc/workflow
          readOnly: true
        - name: data
          mountPath: /data
        
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        
        livenessProbe:
          httpGet:
            path: /health/live
            port: http
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        
        readinessProbe:
          httpGet:
            path: /health/ready
            port: http
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
      
      volumes:
      - name: config
        configMap:
          name: workflow-config
      - name: data
        persistentVolumeClaim:
          claimName: workflow-data
      
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - workflow-system
              topologyKey: kubernetes.io/hostname
```

## 10. Monitoring and Observability

### 10.1 Metrics Collection

```go
// internal/monitoring/metrics.go
package monitoring

import (
    "github.com/prometheus/client_golang/prometheus"
    "github.com/prometheus/client_golang/prometheus/promauto"
)

var (
    // Workflow metrics
    WorkflowsStarted = promauto.NewCounterVec(prometheus.CounterOpts{
        Name: "workflow_system_workflows_started_total",
        Help: "Total number of workflows started",
    }, []string{"type"})
    
    WorkflowsCompleted = promauto.NewCounterVec(prometheus.CounterOpts{
        Name: "workflow_system_workflows_completed_total",
        Help: "Total number of workflows completed",
    }, []string{"type", "status"})
    
    WorkflowDuration = promauto.NewHistogramVec(prometheus.HistogramOpts{
        Name: "workflow_system_workflow_duration_seconds",
        Help: "Duration of workflow execution",
        Buckets: prometheus.DefBuckets,
    }, []string{"type", "agent"})
    
    // Agent metrics
    AgentWorkload = promauto.NewGaugeVec(prometheus.GaugeOpts{
        Name: "workflow_system_agent_workload",
        Help: "Current workload of agents",
    }, []string{"agent_id", "agent_type"})
    
    AgentTaskDuration = promauto.NewHistogramVec(prometheus.HistogramOpts{
        Name: "workflow_system_agent_task_duration_seconds",
        Help: "Duration of agent task execution",
        Buckets: []float64{.1, .25, .5, 1, 2.5, 5, 10, 30, 60},
    }, []string{"agent_type", "task"})
    
    // Database metrics
    DBConnections = promauto.NewGaugeVec(prometheus.GaugeOpts{
        Name: "workflow_system_database_connections",
        Help: "Number of database connections",
    }, []string{"database"})
    
    DBQueryDuration = promauto.NewHistogramVec(prometheus.HistogramOpts{
        Name: "workflow_system_database_query_duration_seconds",
        Help: "Duration of database queries",
        Buckets: prometheus.DefBuckets,
    }, []string{"database", "operation"})
)
```

### 10.2 Distributed Tracing

```go
// internal/tracing/tracer.go
package tracing

import (
    "context"
    "go.opentelemetry.io/otel"
    "go.opentelemetry.io/otel/attribute"
    "go.opentelemetry.io/otel/trace"
)

type Tracer struct {
    tracer trace.Tracer
}

func NewTracer(serviceName string) *Tracer {
    return &Tracer{
        tracer: otel.Tracer(serviceName),
    }
}

func (t *Tracer) StartWorkflow(ctx context.Context, workflowID, workflowType string) (context.Context, trace.Span) {
    ctx, span := t.tracer.Start(ctx, "workflow",
        trace.WithAttributes(
            attribute.String("workflow.id", workflowID),
            attribute.String("workflow.type", workflowType),
        ),
    )
    return ctx, span
}

func (t *Tracer) TraceActivity(ctx context.Context, activityName string) (context.Context, trace.Span) {
    ctx, span := t.tracer.Start(ctx, "activity",
        trace.WithAttributes(
            attribute.String("activity.name", activityName),
        ),
    )
    return ctx, span
}

func (t *Tracer) TraceAgentTask(ctx context.Context, agentID, agentType string) (context.Context, trace.Span) {
    ctx, span := t.tracer.Start(ctx, "agent-task",
        trace.WithAttributes(
            attribute.String("agent.id", agentID),
            attribute.String("agent.type", agentType),
        ),
    )
    return ctx, span
}
```

## 11. Testing Strategy

### 11.1 Testing Levels

| Level | Scope | Tools | Frequency | Responsibility |
|-------|-------|-------|-----------|----------------|
| **Unit** | Individual functions | `go test`, `testify` | Every commit | Developers |
| **Integration** | Component interactions | `dockertest`, `testcontainers` | PR merge | Developers |
| **E2E** | Full workflow scenarios | `cucumber`, `playwright` | Daily | QA |
| **Load** | Performance under load | `k6`, `locust` | Weekly | DevOps |
| **Chaos** | Failure recovery | `chaos-mesh` | Monthly | SRE |

### 11.2 Integration Test Example

```go
// tests/integration/workflow_test.go
package integration

import (
    "context"
    "testing"
    "time"
    
    "github.com/stretchr/testify/require"
    "github.com/ory/dockertest/v3"
    
    "github.com/your-org/beads-workflow-system/internal/bridge"
)

func TestWorkflowLifecycle(t *testing.T) {
    // Setup test environment
    pool, resource := setupTestDatabase(t)
    defer pool.Purge(resource)
    
    // Create bridge
    bridge, cleanup := createTestBridge(t, resource)
    defer cleanup()
    
    // Test: Create workflow
    t.Run("CreateWorkflow", func(t *testing.T) {
        ctx := context.Background()
        req := &bridge.StartWorkflowRequest{
            IssueTitle:    "Test research workflow",
            WorkflowType:  "research",
            AgentType:     "research",
            Variables: map[string]interface{}{
                "query": "tokio async",
            },
        }
        
        workflow, err := bridge.StartWorkflow(ctx, req)
        require.NoError(t, err)
        require.NotNil(t, workflow)
        require.NotEmpty(t, workflow.ID)
        require.NotEmpty(t, workflow.BeadsIssueID)
    })
    
    // Test: Get workflow status
    t.Run("GetWorkflow", func(t *testing.T) {
        ctx := context.Background()
        
        workflow, err := bridge.GetWorkflow(ctx, workflow.ID)
        require.NoError(t, err)
        require.Equal(t, workflow.ID, workflow.ID)
    })
    
    // Test: Complete workflow
    t.Run("CompleteWorkflow", func(t *testing.T) {
        ctx := context.Background()
        
        err := bridge.UpdateWorkflowStatus(ctx, workflow.ID, "completed")
        require.NoError(t, err)
        
        workflow, err := bridge.GetWorkflow(ctx, workflow.ID)
        require.NoError(t, err)
        require.Equal(t, "completed", workflow.Status)
    })
}

func setupTestDatabase(t *testing.T) (*dockertest.Pool, *dockertest.Resource) {
    pool, err := dockertest.NewPool("")
    require.NoError(t, err)
    
    resource, err := pool.Run("alpine", "latest", []string{})
    require.NoError(t, err)
    
    return pool, resource
}
```

## 12. Migration Strategy

### 12.1 Database Migrations

```go
// internal/database/migrations.go
package database

import (
    "context"
    "database/sql"
    "fmt"
)

type Migration struct {
    Version string
    Name    string
    Up      string
    Down    string
}

var Migrations = []Migration{
    {
        Version: "1.0.0",
        Name:    "Initial schema",
        Up: `
            CREATE TABLE workflow_mappings (
                beads_issue_id TEXT PRIMARY KEY,
                tempolite_workflow_id TEXT NOT NULL UNIQUE,
                workflow_type TEXT NOT NULL,
                status TEXT DEFAULT 'active',
                priority INTEGER DEFAULT 2,
                metadata JSON,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE agent_assignments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workflow_id TEXT NOT NULL,
                agent_type TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                step_number INTEGER DEFAULT 1,
                status TEXT DEFAULT 'assigned',
                assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        `,
        Down: `
            DROP TABLE IF EXISTS workflow_mappings;
            DROP TABLE IF EXISTS agent_assignments;
        `,
    },
    {
        Version: "1.1.0",
        Name:    "Add workflow results table",
        Up: `
            CREATE TABLE workflow_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workflow_id TEXT NOT NULL,
                agent_type TEXT NOT NULL,
                result_type TEXT NOT NULL,
                result_data JSON NOT NULL,
                confidence_score REAL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX idx_workflow_results_workflow ON workflow_results(workflow_id);
        `,
        Down: `
            DROP TABLE IF EXISTS workflow_results;
        `,
    },
}

func RunMigrations(db *sql.DB) error {
    // Create migrations table
    _, err := db.Exec(`
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            name TEXT NOT NULL
        )
    `)
    if err != nil {
        return fmt.Errorf("failed to create migrations table: %w", err)
    }
    
    // Get current version
    var currentVersion string
    row := db.QueryRow("SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1")
    row.Scan(&currentVersion) // Ignore error if no migrations
    
    // Apply pending migrations
    for _, migration := range Migrations {
        if migration.Version <= currentVersion {
            continue
        }
        
        if _, err := db.Exec(migration.Up); err != nil {
            return fmt.Errorf("failed to apply migration %s: %w", migration.Version, err)
        }
        
        if _, err := db.Exec("INSERT INTO schema_migrations (version, name) VALUES (?, ?)",
            migration.Version, migration.Name); err != nil {
            return fmt.Errorf("failed to record migration %s: %w", migration.Version, err)
        }
    }
    
    return nil
}
```

## 13. Troubleshooting Guide

### 13.1 Common Issues

**Issue: Database is locked**
- **Symptoms**: Error "database is locked" or timeouts
- **Root Cause**: Concurrent write operations on SQLite
- **Solution**: Use connection pooling, WAL mode, or retry with backoff

**Issue: Workflow stuck in "in_progress"**
- **Symptoms**: Workflow remains in progress indefinitely
- **Root Cause**: Agent crashed or heartbeat timeout
- **Solution**: Check agent health, manually fail workflow, or restart agent

**Issue: Beads sync conflicts**
- **Symptoms**: Git merge conflicts in issues.jsonl
- **Root Cause**: Multiple agents modifying same issue
- **Solution**: Use automatic merge strategy, keep all operations

## 14. Appendix

### 14.1 Glossary

- **Beads**: Git-backed issue tracker for AI agents
- **Tempolite**: SQLite-based workflow execution engine
- **Coordination Bridge**: Integration layer between Beads and Tempolite
- **Activity**: Single unit of work in a workflow
- **Saga**: Long-running transaction with compensation
- **Agent**: Specialized service that executes workflow steps

### 14.2 References

- Beads GitHub Repository: https://github.com/steveyegge/beads
- Tempolite GitHub Repository: https://github.com/davidroman0o/tempolite
- SQLite Documentation: https://www.sqlite.org/docs.html
- Go Documentation: https://golang.org/doc/

---

**Version**: 1.0.0  
**Last Updated**: 2026-02-07  
**Status**: Complete