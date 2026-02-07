# API Design Overview

## 🔌 API Architecture

The workflow system provides a **layered API architecture** with three main interfaces:

```
┌─────────────────────────────────────────────────┐
│                API ARCHITECTURE               │
│                                                 │
│  ┌─────────────────┐    ┌─────────────────┐   │
│  │   REST API     │    │   CLI API      │   │
│  │                 │    │                 │   │
│  │ • HTTP/HTTPS   │    │ • Command line  │   │
│  │ • JSON format  │    │ • Human readable│   │
│  │ • Browser UI   │    │ • Automation    │   │
│  └─────────────────┘    └─────────────────┘   │
│           │                      │             │
│           └──────────────┬─────────┘             │
│                          ▼                         │
│  ┌─────────────────────────────────────────┐   │
│  │         INTERNAL API                 │   │
│  │                                     │   │
│  │ • CoordinationBridge                │   │
│  │ • BeadsClient                     │   │
│  │ • TempoliteEngine                 │   │
│  │ • DatabaseManager                 │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
└─────────────────────────────────────────────────┘
```

## 🌐 REST API Specification

### Base Configuration

```yaml
# config/api.yaml
server:
  host: "0.0.0.0"
  port: 8080
  timeout: 30s
  read_timeout: 60s
  write_timeout: 60s
  max_header_bytes: 1048576

cors:
  allowed_origins: ["http://localhost:3000", "https://yourdomain.com"]
  allowed_methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
  allowed_headers: ["Content-Type", "Authorization", "X-Request-ID"]
  
auth:
  enabled: true
  jwt_secret: "${JWT_SECRET}"
  token_expiry: "24h"
  refresh_token_expiry: "168h" # 7 days

rate_limit:
  enabled: true
  requests_per_minute: 100
  burst: 20
```

### Core Endpoints

#### Workflow Management

```http
### Start Workflow
POST /api/v1/workflows

Request:
{
  "issue_title": "Research async Rust frameworks",
  "workflow_type": "research",
  "agent_type": "research",
  "priority": 1,
  "variables": {
    "query": "async rust frameworks",
    "focus": "performance",
    "libraries": ["tokio", "async-std", "smol"]
  },
  "template_id": "research-template-v1"
}

Response (201):
{
  "workflow_id": "wf-research-001",
  "beads_issue_id": "bd-a1b2",
  "status": "started",
  "agent_id": "research-agent-01",
  "created_at": "2026-02-07T10:30:00Z",
  "estimated_duration": "15m",
  "next_steps": [
    {
      "step": "library_discovery",
      "estimated_time": "5m",
      "description": "Discover and analyze relevant libraries"
    },
    {
      "step": "documentation_analysis", 
      "estimated_time": "8m",
      "description": "Parse and analyze documentation"
    },
    {
      "step": "findings_synthesis",
      "estimated_time": "2m", 
      "description": "Synthesize research findings"
    }
  ]
}

### Get Workflow Status
GET /api/v1/workflows/{workflow_id}

Response (200):
{
  "workflow_id": "wf-research-001",
  "beads_issue_id": "bd-a1b2",
  "status": "in_progress",
  "agent_type": "research",
  "agent_id": "research-agent-01",
  "progress": {
    "current_step": "documentation_analysis",
    "step_number": 2,
    "total_steps": 3,
    "completion_percentage": 65.0
  },
  "results": {
    "libraries_found": 3,
    "documents_analyzed": 2,
    "findings_generated": 1
  },
  "timing": {
    "started_at": "2026-02-07T10:30:00Z",
    "current_step_started_at": "2026-02-07T10:35:00Z",
    "estimated_completion": "2026-02-07T10:45:00Z",
    "elapsed_time_ms": 300000
  }
}

### List Workflows
GET /api/v1/workflows?status=active&agent_type=research&limit=50&offset=0

Response (200):
{
  "workflows": [
    {
      "workflow_id": "wf-research-001",
      "beads_issue_id": "bd-a1b2",
      "title": "Research async Rust frameworks",
      "status": "in_progress", 
      "agent_type": "research",
      "priority": 1,
      "created_at": "2026-02-07T10:30:00Z",
      "updated_at": "2026-02-07T10:42:15Z"
    }
  ],
  "pagination": {
    "total": 127,
    "limit": 50,
    "offset": 0,
    "has_more": true
  }
}

### Cancel Workflow
POST /api/v1/workflows/{workflow_id}/cancel

Request:
{
  "reason": "User request - priority change",
  "save_progress": true
}

Response (200):
{
  "workflow_id": "wf-research-001",
  "status": "cancelled",
  "cancelled_at": "2026-02-07T10:48:22Z",
  "progress_saved": true,
  "cleanup_status": "completed"
}
```

#### Agent Management

```http
### Register Agent
POST /api/v1/agents

Request:
{
  "agent_id": "research-agent-01",
  "agent_type": "research",
  "capabilities": {
    "library_discovery": ["rust", "python", "javascript", "go"],
    "documentation_parsing": ["markdown", "html", "pdf"],
    "static_analysis": ["rust", "python"],
    "benchmarking": ["performance", "memory", "throughput"]
  },
  "configuration": {
    "max_workload": 5,
    "timeout": "30m",
    "retry_policy": "exponential_backoff",
    "resource_limits": {
      "memory_mb": 2048,
      "cpu_percent": 80,
      "disk_io_mb_s": 100
    }
  },
  "endpoints": {
    "health_check": "http://localhost:9001/health",
    "task_assignment": "http://localhost:9001/tasks"
  }
}

Response (201):
{
  "agent_id": "research-agent-01",
  "status": "registered",
  "registered_at": "2026-02-07T10:00:00Z",
  "initial_workload": 0,
  "capabilities_verified": true
}

### Get Agent Status
GET /api/v1/agents/{agent_id}/status

Response (200):
{
  "agent_id": "research-agent-01",
  "agent_type": "research",
  "status": "active",
  "current_workload": 2,
  "max_workload": 5,
  "workload_percentage": 40.0,
  "current_tasks": [
    {
      "workflow_id": "wf-research-001",
      "step": "documentation_analysis",
      "started_at": "2026-02-07T10:35:00Z",
      "progress_percentage": 75.0
    }
  ],
  "last_heartbeat": "2026-02-07T10:47:30Z",
  "performance_metrics": {
    "tasks_completed_today": 8,
    "avg_task_duration_ms": 180000,
    "success_rate": 95.5,
    "error_rate": 4.5
  }
}

### List Agents
GET /api/v1/agents?type=research&status=active

Response (200):
{
  "agents": [
    {
      "agent_id": "research-agent-01", 
      "agent_type": "research",
      "status": "active",
      "current_workload": 2,
      "max_workload": 5,
      "workload_percentage": 40.0,
      "last_heartbeat": "2026-02-07T10:47:30Z"
    },
    {
      "agent_id": "research-agent-02",
      "agent_type": "research", 
      "status": "active",
      "current_workload": 1,
      "max_workload": 5,
      "workload_percentage": 20.0,
      "last_heartbeat": "2026-02-07T10:46:15Z"
    }
  ],
  "summary": {
    "total_agents": 2,
    "active_agents": 2,
    "total_capacity": 10,
    "used_capacity": 3,
    "available_capacity": 7
  }
}
```

#### Results and Analytics

```http
### Get Workflow Results
GET /api/v1/workflows/{workflow_id}/results

Response (200):
{
  "workflow_id": "wf-research-001",
  "results": [
    {
      "result_type": "findings",
      "agent_type": "research",
      "confidence_score": 0.85,
      "quality_score": 8.2,
      "execution_time_ms": 180000,
      "data": {
        "libraries_analyzed": [
          {
            "name": "tokio",
            "version": "1.0.0",
            "score": 9.2,
            "findings": [
              {
                "type": "performance",
                "description": "Excellent async performance with low overhead",
                "evidence": ["benchmarks", "community_usage"]
              }
            ]
          }
        ],
        "summary": "Tokio recommended for production use"
      },
      "artifacts": [
        "research_findings.json",
        "library_comparisons.csv", 
        "recommendations.md"
      ],
      "created_at": "2026-02-07T10:45:00Z"
    }
  ]
}

### Get Performance Analytics
GET /api/v1/analytics/performance?period=7d&agent_type=research

Response (200):
{
  "period": "7d",
  "analytics": {
    "workflow_metrics": {
      "total_workflows": 42,
      "completed_workflows": 38,
      "failed_workflows": 4,
      "success_rate": 90.5
    },
    "performance_metrics": {
      "avg_execution_time_ms": 165000,
      "p95_execution_time_ms": 240000,
      "p99_execution_time_ms": 320000,
      "throughput_per_hour": 2.3
    },
    "agent_metrics": {
      "total_agents": 2,
      "active_agents": 2,
      "avg_workload_percentage": 35.5,
      "peak_workload_percentage": 80.0
    },
    "quality_metrics": {
      "avg_confidence_score": 0.82,
      "avg_quality_score": 7.8,
      "error_rate_percentage": 9.5
    },
    "resource_usage": {
      "avg_memory_mb": 1536,
      "peak_memory_mb": 2048,
      "avg_cpu_percent": 45.2,
      "peak_cpu_percent": 78.5
    }
  },
  "trends": [
    {
      "date": "2026-02-07",
      "workflow_count": 6,
      "success_rate": 91.7,
      "avg_execution_time_ms": 158000
    }
  ]
}
```

## 🔧 Internal API

### CoordinationBridge Interface

```go
type CoordinationBridge interface {
    // Workflow lifecycle
    StartWorkflow(ctx context.Context, req *StartWorkflowRequest) (*Workflow, error)
    GetWorkflow(ctx context.Context, workflowID string) (*Workflow, error)
    UpdateWorkflowStatus(ctx context.Context, workflowID, status string) error
    CancelWorkflow(ctx context.Context, workflowID, reason string) error
    
    // Agent management
    RegisterAgent(ctx context.Context, agent *Agent) error
    UnregisterAgent(ctx context.Context, agentID string) error
    GetAgentStatus(ctx context.Context, agentID string) (*AgentStatus, error)
    AssignWorkflow(ctx context.Context, workflowID, agentID string) error
    
    // Results storage
    StoreResults(ctx context.Context, workflowID string, results *Results) error
    GetResults(ctx context.Context, workflowID string) ([]*Results, error)
    
    // Analytics
    GetWorkflowAnalytics(ctx context.Context, filters *AnalyticsFilters) (*Analytics, error)
    GetPerformanceMetrics(ctx context.Context, period time.Duration) (*PerformanceMetrics, error)
}

type StartWorkflowRequest struct {
    IssueTitle      string                 `json:"issue_title"`
    WorkflowType   string                 `json:"workflow_type"`
    AgentType      string                 `json:"agent_type"`
    Priority       int                    `json:"priority"`
    Variables      map[string]interface{}   `json:"variables"`
    TemplateID     string                 `json:"template_id,omitempty"`
}

type Workflow struct {
    ID              string                 `json:"id"`
    BeadsIssueID    string                 `json:"beads_issue_id"`
    Type            string                 `json:"type"`
    Status          string                 `json:"status"`
    Priority        int                    `json:"priority"`
    AgentID        string                 `json:"agent_id,omitempty"`
    CurrentStep     string                 `json:"current_step,omitempty"`
    StepNumber      int                    `json:"step_number,omitempty"`
    TotalSteps      int                    `json:"total_steps,omitempty"`
    ProgressPercent float64                `json:"progress_percent,omitempty"`
    StartedAt       time.Time              `json:"started_at"`
    EstimatedEnd    time.Time              `json:"estimated_end,omitempty"`
    Variables       map[string]interface{} `json:"variables,omitempty"`
    Results         []*Result              `json:"results,omitempty"`
}
```

### BeadsClient Interface

```go
type BeadsClient interface {
    // Issue management
    CreateIssue(ctx context.Context, req *CreateIssueRequest) (*Issue, error)
    GetIssue(ctx context.Context, issueID string) (*Issue, error)
    UpdateIssue(ctx context.Context, issueID string, updates map[string]interface{}) error
    CloseIssue(ctx context.Context, issueID string, reason string) error
    
    // Dependencies
    AddDependency(ctx context.Context, parentID, childID, depType string) error
    RemoveDependency(ctx context.Context, parentID, childID string) error
    GetDependencies(ctx context.Context, issueID string) ([]*Dependency, error)
    
    // Multi-agent
    AssignIssue(ctx context.Context, issueID, agentID string) error
    AddComment(ctx context.Context, issueID, comment string) error
    GetReadyWork(ctx context.Context, filters *WorkFilters) ([]*Issue, error)
    
    // Formulas and molecules
    PourFormula(ctx context.Context, formulaName string, variables map[string]string) (*Molecule, error)
    ListFormulas(ctx context.Context) ([]*Formula, error)
    GetMolecules(ctx context.Context, filters *MoleculeFilters) ([]*Molecule, error)
}

type CreateIssueRequest struct {
    Title       string   `json:"title"`
    Description string   `json:"description,omitempty"`
    Type        string   `json:"type,omitempty"`
    Priority    int      `json:"priority,omitempty"`
    Assignee    string   `json:"assignee,omitempty"`
    Labels      []string `json:"labels,omitempty"`
    ParentID    string   `json:"parent_id,omitempty"`
}
```

### TempoliteEngine Interface

```go
type TempoliteEngine interface {
    // Workflow execution
    StartWorkflow(ctx context.Context, workflowID string, definition interface{}) error
    ExecuteActivity(ctx context.Context, workflowID, activityName string, params interface{}) (interface{}, error)
    ExecuteSaga(ctx context.Context, workflowID, sagaID string, saga *Saga) error
    
    // Signaling
    Signal(ctx context.Context, workflowID, signalName string, data interface{}) error
    WaitForSignal(ctx context.Context, workflowID, signalName string) (interface{}, error)
    
    // Checkpoints and recovery
    CreateCheckpoint(ctx context.Context, workflowID string) error
    RestoreFromCheckpoint(ctx context.Context, workflowID string) error
    GetCheckpoints(ctx context.Context, workflowID string) ([]*Checkpoint, error)
    
    // Performance monitoring
    GetWorkflowMetrics(ctx context.Context, workflowID string) (*WorkflowMetrics, error)
    GetActivityMetrics(ctx context.Context, activityName string) (*ActivityMetrics, error)
}

type Saga struct {
    ID       string
    Steps    []*SagaStep
    Metadata map[string]interface{}
}

type SagaStep struct {
    Transaction func(TransactionContext) error
    Compensate func(CompensationContext) error
    Metadata    map[string]interface{}
}
```

## 🚦 CLI API Design

### Command Structure

```bash
# Workflow management
workflow start <type> <title> [flags]
workflow status <workflow-id>
workflow list [flags]
workflow cancel <workflow-id> [flags]
workflow results <workflow-id>

# Agent management
agent register <agent-config>
agent status <agent-id>
agent list [flags]
agent assign <workflow-id> <agent-id>

# Templates and formulas
template list [flags]
template show <template-id>
template create <template-file>
workflow use-template <template-id> [variables]

# Analytics and monitoring
analytics performance [flags]
analytics agents [flags]
analytics workflows [flags]
workflow logs <workflow-id> [flags]
```

### Example CLI Usage

```bash
# Start research workflow
workflow start research "Analyze Rust async frameworks" \
  --priority 1 \
  --variable "focus=performance" \
  --variable "libraries=tokio,async-std,smol" \
  --template research-v1

# Check workflow status
workflow status wf-research-001

# Get detailed results
workflow results wf-research-001 --format json --output results.json

# List active workflows
workflow list --status active --agent-type research

# Register new agent
agent register --config agents/research-agent.yaml

# Get agent workload
agent status research-agent-01 --detailed

# Assign workflow to agent
agent assign wf-research-001 research-agent-02 --reason "load_balancing"

# Get performance analytics
analytics performance --period 7d --agent-type research --format table

# Get workflow logs
workflow logs wf-research-001 --level debug --since 1h --follow
```

## 🔐 Authentication & Authorization

### JWT Token Format

```json
{
  "sub": "user-123",
  "agent_id": "research-agent-01", 
  "agent_type": "research",
  "capabilities": [
    "workflow:start",
    "workflow:read",
    "results:read",
    "analytics:read"
  ],
  "permissions": {
    "workflows": ["read", "write", "execute"],
    "agents": ["read"],
    "results": ["read", "write"],
    "analytics": ["read"]
  },
  "iat": 1644230400,
  "exp": 1644316800,
  "jti": "token-id-123"
}
```

### Authorization Matrix

| Role | Workflows | Agents | Results | Analytics | System |
|-------|------------|---------|---------|-----------|---------|
| admin | CRUD | CRUD | CRUD | CRUD | CRUD |
| agent | R* | R | CRW | R | - |
| viewer | R | R | R | R | - |

*R: Agents can only read/modify their own assigned workflows

## 📊 Rate Limiting

### Implementation

```go
type RateLimiter struct {
    store    *redis.Client  // or in-memory for development
    requests map[string]*rate.Limiter
    mu       sync.RWMutex
}

func (rl *RateLimiter) Allow(key string, limit int, window time.Duration) bool {
    rl.mu.Lock()
    defer rl.mu.Unlock()
    
    limiter, exists := rl.requests[key]
    if !exists {
        limiter = rate.NewLimiter(rate.Every(window/time.Duration(limit)), limit)
        rl.requests[key] = limiter
    }
    
    return limiter.Allow()
}

// Rate limits per endpoint
var RateLimits = map[string]RateLimit{
    "workflow:start":     {Limit: 10, Window: time.Minute},
    "workflow:status":    {Limit: 100, Window: time.Minute},
    "agent:register":     {Limit: 5, Window: time.Hour},
    "analytics:performance": {Limit: 20, Window: time.Minute},
}
```

## 🔄 Error Handling

### Standard Error Response Format

```json
{
  "error": {
    "code": "WORKFLOW_NOT_FOUND",
    "message": "Workflow 'wf-research-001' not found",
    "details": {
      "workflow_id": "wf-research-001",
      "search_time_ms": 15
    },
    "request_id": "req-123456",
    "timestamp": "2026-02-07T10:48:22Z"
  }
}
```

### Error Codes

| Code | HTTP Status | Description |
|-------|-------------|-------------|
| WORKFLOW_NOT_FOUND | 404 | Workflow not found |
| AGENT_UNAVAILABLE | 503 | No agents available for workflow type |
| INVALID_WORKFLOW_TYPE | 400 | Unsupported workflow type |
| RATE_LIMIT_EXCEEDED | 429 | Rate limit exceeded |
| AUTHENTICATION_FAILED | 401 | Invalid authentication |
| PERMISSION_DENIED | 403 | Insufficient permissions |
| SYSTEM_ERROR | 500 | Internal system error |

## 🔗 Cross-References

- [System Design](../architecture/01-system-design.md) - Overall system architecture
- [Database Schema](../architecture/02-database-schema.md) - Database design
- [Security Model](../architecture/04-security.md) - Security architecture
- [CLI Commands](../cli-commands/01-reference.md) - Complete CLI reference
- [Implementation Guide](../implementation/01-setup.md) - Setup and installation

---

**Next**: Read the [CLI Commands](../cli-commands/01-reference.md) documentation.