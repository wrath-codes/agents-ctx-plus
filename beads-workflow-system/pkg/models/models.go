package models

import (
	"time"
)

// Workflow represents a workflow execution
type Workflow struct {
	ID              string                 `json:"id"`
	BeadsIssueID    string                 `json:"beads_issue_id"`
	Type            string                 `json:"type"`
	Status          string                 `json:"status"`
	Priority        int                    `json:"priority"`
	AgentID         string                 `json:"agent_id,omitempty"`
	CurrentStep     string                 `json:"current_step,omitempty"`
	StepNumber      int                    `json:"step_number,omitempty"`
	TotalSteps      int                    `json:"total_steps,omitempty"`
	ProgressPercent float64                `json:"progress_percent,omitempty"`
	StartedAt       time.Time              `json:"started_at"`
	CompletedAt     *time.Time             `json:"completed_at,omitempty"`
	EstimatedEnd    *time.Time             `json:"estimated_end,omitempty"`
	Variables       map[string]interface{} `json:"variables,omitempty"`
	Results         []*Result              `json:"results,omitempty"`
	Metadata        map[string]interface{} `json:"metadata,omitempty"`
}

// Result represents a workflow execution result
type Result struct {
	ID              string                 `json:"id"`
	WorkflowID      string                 `json:"workflow_id"`
	AgentType       string                 `json:"agent_type"`
	ResultType      string                 `json:"result_type"`
	Data            map[string]interface{} `json:"data"`
	ConfidenceScore float64                `json:"confidence_score"`
	QualityScore    float64                `json:"quality_score"`
	ExecutionTimeMs int                    `json:"execution_time_ms"`
	Artifacts       []string               `json:"artifacts,omitempty"`
	CreatedAt       time.Time              `json:"created_at"`
}

// Agent represents a workflow agent
type Agent struct {
	ID               string                 `json:"id"`
	Type             string                 `json:"type"`
	Status           string                 `json:"status"`
	Capabilities     map[string][]string    `json:"capabilities"`
	Configuration    map[string]interface{} `json:"configuration"`
	MaxWorkload      int                    `json:"max_workload"`
	CurrentWorkload  int                    `json:"current_workload"`
	LastHeartbeat    time.Time              `json:"last_heartbeat"`
	PerformanceStats *PerformanceStats      `json:"performance_stats,omitempty"`
	Endpoints        map[string]string      `json:"endpoints,omitempty"`
}

// PerformanceStats tracks agent performance metrics
type PerformanceStats struct {
	TasksCompletedToday int     `json:"tasks_completed_today"`
	AvgTaskDurationMs   int     `json:"avg_task_duration_ms"`
	SuccessRate         float64 `json:"success_rate"`
	ErrorRate           float64 `json:"error_rate"`
}

// AgentAssignment tracks workflow assignments to agents
type AgentAssignment struct {
	ID          int        `json:"id"`
	WorkflowID  string     `json:"workflow_id"`
	AgentType   string     `json:"agent_type"`
	AgentID     string     `json:"agent_id"`
	StepNumber  int        `json:"step_number"`
	StepName    string     `json:"step_name"`
	AssignedAt  time.Time  `json:"assigned_at"`
	StartedAt   *time.Time `json:"started_at,omitempty"`
	CompletedAt *time.Time `json:"completed_at,omitempty"`
	Status      string     `json:"status"`
	HandoffFrom string     `json:"handoff_from,omitempty"`
	HandoffTo   string     `json:"handoff_to,omitempty"`
}

// WorkflowMapping links beads issues to tempolite workflows
type WorkflowMapping struct {
	BeadsIssueID        string                 `json:"beads_issue_id"`
	TempoliteWorkflowID string                 `json:"tempolite_workflow_id"`
	WorkflowType        string                 `json:"workflow_type"`
	Status              string                 `json:"status"`
	Priority            int                    `json:"priority"`
	Metadata            map[string]interface{} `json:"metadata,omitempty"`
	ParentWorkflowID    string                 `json:"parent_workflow_id,omitempty"`
	CreatedAt           time.Time              `json:"created_at"`
	UpdatedAt           time.Time              `json:"updated_at"`
	CompletedAt         *time.Time             `json:"completed_at,omitempty"`
}

// Issue represents a beads issue
type Issue struct {
	ID          string                 `json:"id"`
	Title       string                 `json:"title"`
	Description string                 `json:"description,omitempty"`
	Type        string                 `json:"type,omitempty"`
	Priority    int                    `json:"priority,omitempty"`
	Status      string                 `json:"status"`
	Assignee    string                 `json:"assignee,omitempty"`
	Labels      []string               `json:"labels,omitempty"`
	ParentID    string                 `json:"parent_id,omitempty"`
	CreatedAt   time.Time              `json:"created_at"`
	UpdatedAt   time.Time              `json:"updated_at"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// Dependency represents a dependency between issues
type Dependency struct {
	ID          string    `json:"id"`
	IssueID     string    `json:"issue_id"`
	DependsOnID string    `json:"depends_on_id"`
	DepType     string    `json:"dep_type"`
	CreatedAt   time.Time `json:"created_at,omitempty"`
}

// IssueFilter represents filters for listing/searching issues
type IssueFilter struct {
	Status   string   `json:"status,omitempty"`
	Type     string   `json:"type,omitempty"`
	Priority int      `json:"priority,omitempty"`
	Assignee string   `json:"assignee,omitempty"`
	Labels   []string `json:"labels,omitempty"`
	Search   string   `json:"search,omitempty"`
	Limit    int      `json:"limit,omitempty"`
}

// UpdateIssueRequest represents a partial update to an issue
type UpdateIssueRequest struct {
	Title       string `json:"title,omitempty"`
	Description string `json:"description,omitempty"`
	Type        string `json:"type,omitempty"`
	Priority    int    `json:"priority,omitempty"`
	Status      string `json:"status,omitempty"`
	Assignee    string `json:"assignee,omitempty"`
	ParentID    string `json:"parent_id,omitempty"`
}

// BlockedIssue represents an issue that is blocked by dependencies
type BlockedIssue struct {
	IssueID   string   `json:"issue_id"`
	Title     string   `json:"title"`
	Status    string   `json:"status"`
	BlockedBy []string `json:"blocked_by,omitempty"`
}

// WorkflowTemplate represents a reusable workflow template
type WorkflowTemplate struct {
	ID            string                 `json:"id"`
	Name          string                 `json:"name"`
	Description   string                 `json:"description,omitempty"`
	Type          string                 `json:"type"`
	Definition    map[string]interface{} `json:"definition"`
	AgentSequence []string               `json:"agent_sequence"`
	Variables     map[string]interface{} `json:"variables,omitempty"`
	Version       string                 `json:"version"`
	IsActive      bool                   `json:"is_active"`
	UsageCount    int                    `json:"usage_count"`
	SuccessRate   float64                `json:"success_rate"`
	CreatedAt     time.Time              `json:"created_at"`
	UpdatedAt     time.Time              `json:"updated_at"`
}

// PerformanceMetrics tracks workflow performance
type PerformanceMetrics struct {
	ID              int       `json:"id"`
	WorkflowID      string    `json:"workflow_id"`
	AgentType       string    `json:"agent_type"`
	StepName        string    `json:"step_name"`
	StartTime       time.Time `json:"start_time"`
	EndTime         time.Time `json:"end_time"`
	DurationMs      int       `json:"duration_ms"`
	Success         bool      `json:"success"`
	ErrorMessage    string    `json:"error_message,omitempty"`
	MemoryUsageMb   int       `json:"memory_usage_mb"`
	CpuUsagePercent float64   `json:"cpu_usage_percent"`
}

// Saga represents a transactional saga workflow
type Saga struct {
	ID       string                 `json:"id"`
	Steps    []*SagaStep            `json:"steps"`
	Metadata map[string]interface{} `json:"metadata,omitempty"`
}

// SagaStep represents a single step in a saga
type SagaStep struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Transaction func(TransactionContext) error
	Compensate  func(CompensationContext) error
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// TransactionContext provides context for saga transactions
type TransactionContext struct {
	SagaID     string
	StepID     string
	WorkflowID string
	Data       map[string]interface{}
}

// CompensationContext provides context for saga compensations
type CompensationContext struct {
	SagaID     string
	StepID     string
	WorkflowID string
	Data       map[string]interface{}
}

// Checkpoint represents a workflow checkpoint for recovery
type Checkpoint struct {
	ID         string                 `json:"id"`
	WorkflowID string                 `json:"workflow_id"`
	StepNumber int                    `json:"step_number"`
	State      map[string]interface{} `json:"state"`
	CreatedAt  time.Time              `json:"created_at"`
}

// StartWorkflowRequest represents a request to start a workflow
type StartWorkflowRequest struct {
	IssueTitle   string                 `json:"issue_title"`
	WorkflowType string                 `json:"workflow_type"`
	AgentType    string                 `json:"agent_type"`
	Priority     int                    `json:"priority"`
	Variables    map[string]interface{} `json:"variables,omitempty"`
	TemplateID   string                 `json:"template_id,omitempty"`
}

// CreateIssueRequest represents a request to create a beads issue
type CreateIssueRequest struct {
	Title       string   `json:"title"`
	Description string   `json:"description,omitempty"`
	Type        string   `json:"type,omitempty"`
	Priority    int      `json:"priority,omitempty"`
	Assignee    string   `json:"assignee,omitempty"`
	Labels      []string `json:"labels,omitempty"`
	ParentID    string   `json:"parent_id,omitempty"`
}

// AnalyticsFilters represents filters for analytics queries
type AnalyticsFilters struct {
	StartDate    *time.Time `json:"start_date,omitempty"`
	EndDate      *time.Time `json:"end_date,omitempty"`
	WorkflowType string     `json:"workflow_type,omitempty"`
	AgentType    string     `json:"agent_type,omitempty"`
	Status       string     `json:"status,omitempty"`
}

// WorkFilters represents filters for getting ready work
type WorkFilters struct {
	AgentID string `json:"agent_id,omitempty"`
	Type    string `json:"type,omitempty"`
}

// Molecule represents an instantiated workflow (Beads concept)
type Molecule struct {
	ID          string            `json:"id"`
	FormulaName string            `json:"formula_name"`
	Variables   map[string]string `json:"variables,omitempty"`
	Status      string            `json:"status"`
	CreatedAt   time.Time         `json:"created_at"`
	CompletedAt *time.Time        `json:"completed_at,omitempty"`
}

// MoleculeFilters represents filters for molecules
type MoleculeFilters struct {
	FormulaName string `json:"formula_name,omitempty"`
	Status      string `json:"status,omitempty"`
}

// Formula represents a workflow template (Beads concept)
type Formula struct {
	ID          string                 `json:"id"`
	Name        string                 `json:"name"`
	Description string                 `json:"description,omitempty"`
	Definition  map[string]interface{} `json:"definition"`
	IsActive    bool                   `json:"is_active"`
	CreatedAt   time.Time              `json:"created_at"`
}

// Analytics represents workflow analytics data
type Analytics struct {
	Period             string                 `json:"period"`
	TotalWorkflows     int                    `json:"total_workflows"`
	CompletedWorkflows int                    `json:"completed_workflows"`
	FailedWorkflows    int                    `json:"failed_workflows"`
	SuccessRate        float64                `json:"success_rate"`
	AvgExecutionTime   int                    `json:"avg_execution_time_ms"`
	Metrics            map[string]interface{} `json:"metrics,omitempty"`
}

// ErrorResponse represents a standard error response
type ErrorResponse struct {
	Error struct {
		Code      string                 `json:"code"`
		Message   string                 `json:"message"`
		Details   map[string]interface{} `json:"details,omitempty"`
		RequestID string                 `json:"request_id"`
		Timestamp time.Time              `json:"timestamp"`
	} `json:"error"`
}

// WorkflowStatus represents possible workflow statuses
const (
	WorkflowStatusActive    = "active"
	WorkflowStatusCompleted = "completed"
	WorkflowStatusFailed    = "failed"
	WorkflowStatusPaused    = "paused"
	WorkflowStatusCancelled = "cancelled"
)

// AgentStatus represents possible agent statuses
const (
	AgentStatusActive   = "active"
	AgentStatusInactive = "inactive"
	AgentStatusBusy     = "busy"
	AgentStatusError    = "error"
)

// AgentType represents possible agent types
const (
	AgentTypeResearch      = "research"
	AgentTypePOC           = "poc"
	AgentTypeDocumentation = "documentation"
	AgentTypeValidation    = "validation"
	AgentTypeSupervisor    = "supervisor"
)

// WorkflowType represents possible workflow types
const (
	WorkflowTypeResearch      = "research"
	WorkflowTypePOC           = "poc"
	WorkflowTypeDocumentation = "documentation"
	WorkflowTypeValidation    = "validation"
)

// ResultType represents possible result types
const (
	ResultTypeFindings      = "findings"
	ResultTypePOCResults    = "poc_results"
	ResultTypeDocumentation = "documentation"
	ResultTypeValidation    = "validation"
	ResultTypePerformance   = "performance"
)
