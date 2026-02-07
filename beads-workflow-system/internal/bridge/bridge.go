package bridge

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/your-org/beads-workflow-system/internal/beads"
	"github.com/your-org/beads-workflow-system/internal/tempolite"
	"github.com/your-org/beads-workflow-system/pkg/models"
)

// CoordinationBridge bridges Beads and Tempolite systems
type CoordinationBridge struct {
	beadsClient *beads.Client
	tempolite   *tempolite.Engine
	coordDB     *sql.DB
	eventBus    *EventBus
	// Maps our string workflow IDs to tempolite's int WorkflowEntityIDs.
	// Populated when workflows are executed through the tempolite engine.
	entityMap map[string]tempolite.WorkflowEntityID
}

// EventBus handles event distribution
type EventBus struct {
	subscribers map[string][]chan Event
}

// Event represents a system event
type Event struct {
	Type       string
	WorkflowID string
	Data       map[string]interface{}
	Timestamp  time.Time
}

// NewCoordinationBridge creates a new coordination bridge
func NewCoordinationBridge(beadsClient *beads.Client, tempoliteEngine *tempolite.Engine, coordDB *sql.DB) *CoordinationBridge {
	return &CoordinationBridge{
		beadsClient: beadsClient,
		tempolite:   tempoliteEngine,
		coordDB:     coordDB,
		eventBus:    NewEventBus(),
		entityMap:   make(map[string]tempolite.WorkflowEntityID),
	}
}

// MapWorkflowEntity records the mapping between our string workflow ID
// and the tempolite WorkflowEntityID assigned when the workflow was executed.
func (cb *CoordinationBridge) MapWorkflowEntity(workflowID string, entityID tempolite.WorkflowEntityID) {
	cb.entityMap[workflowID] = entityID
}

// GetTempoliteEntityID looks up the tempolite WorkflowEntityID for a string workflow ID.
// Returns (id, true) if found, (0, false) if not.
func (cb *CoordinationBridge) GetTempoliteEntityID(workflowID string) (tempolite.WorkflowEntityID, bool) {
	id, ok := cb.entityMap[workflowID]
	return id, ok
}

// NewEventBus creates a new event bus
func NewEventBus() *EventBus {
	return &EventBus{
		subscribers: make(map[string][]chan Event),
	}
}

// Subscribe subscribes to an event type
func (eb *EventBus) Subscribe(eventType string) chan Event {
	ch := make(chan Event, 100)
	eb.subscribers[eventType] = append(eb.subscribers[eventType], ch)
	return ch
}

// Publish publishes an event
func (eb *EventBus) Publish(event Event) {
	if subs, ok := eb.subscribers[event.Type]; ok {
		for _, ch := range subs {
			select {
			case ch <- event:
			default:
				// Channel full, drop event
			}
		}
	}
}

// StartWorkflow starts a new workflow and creates a beads issue
func (cb *CoordinationBridge) StartWorkflow(ctx context.Context, req *models.StartWorkflowRequest) (*models.Workflow, error) {
	// 1. Create beads issue
	issueReq := &models.CreateIssueRequest{
		Title:    req.IssueTitle,
		Type:     req.WorkflowType,
		Priority: req.Priority,
		Labels:   []string{req.WorkflowType, "workflow"},
	}

	issue, err := cb.beadsClient.CreateIssue(ctx, issueReq)
	if err != nil {
		return nil, fmt.Errorf("failed to create beads issue: %w", err)
	}

	// 2. Generate workflow ID
	workflowID := generateWorkflowID(req.WorkflowType)

	// 3. Create mapping in coordination database
	metadata := map[string]interface{}{
		"variables":   req.Variables,
		"template_id": req.TemplateID,
	}
	metadataJSON, _ := json.Marshal(metadata)

	_, err = cb.coordDB.ExecContext(ctx, `
		INSERT INTO workflow_mappings (beads_issue_id, tempolite_workflow_id, workflow_type, priority, metadata, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
	`, issue.ID, workflowID, req.WorkflowType, req.Priority, metadataJSON, time.Now(), time.Now())

	if err != nil {
		return nil, fmt.Errorf("failed to create workflow mapping: %w", err)
	}

	// 4. Store tempolite entity ID if a workflow function is executed later.
	// The bridge records the mapping so recovery can resume via tempolite.
	// Actual tempolite execution is triggered by ExecuteViaTempolite()
	// after the caller registers and runs the workflow function.

	// 5. Create initial agent assignment
	if req.AgentType != "" {
		_, err = cb.coordDB.ExecContext(ctx, `
			INSERT INTO agent_assignments (workflow_id, agent_type, agent_id, status, assigned_at)
			VALUES (?, ?, ?, 'assigned', ?)
		`, workflowID, req.AgentType, generateAgentID(req.AgentType), time.Now())

		if err != nil {
			return nil, fmt.Errorf("failed to create agent assignment: %w", err)
		}
	}

	// 6. Publish event
	cb.eventBus.Publish(Event{
		Type:       "workflow:started",
		WorkflowID: workflowID,
		Data: map[string]interface{}{
			"beads_issue_id": issue.ID,
			"workflow_type":  req.WorkflowType,
			"agent_type":     req.AgentType,
		},
		Timestamp: time.Now(),
	})

	return &models.Workflow{
		ID:           workflowID,
		BeadsIssueID: issue.ID,
		Type:         req.WorkflowType,
		Status:       models.WorkflowStatusActive,
		Priority:     req.Priority,
		AgentID:      generateAgentID(req.AgentType),
		StartedAt:    time.Now(),
		Variables:    req.Variables,
	}, nil
}

// GetWorkflow retrieves a workflow by ID
func (cb *CoordinationBridge) GetWorkflow(ctx context.Context, workflowID string) (*models.Workflow, error) {
	var workflow models.Workflow
	var metadataJSON string

	err := cb.coordDB.QueryRowContext(ctx, `
		SELECT beads_issue_id, workflow_type, status, priority, metadata, created_at
		FROM workflow_mappings WHERE tempolite_workflow_id = ?
	`, workflowID).Scan(&workflow.BeadsIssueID, &workflow.Type, &workflow.Status,
		&workflow.Priority, &metadataJSON, &workflow.StartedAt)

	if err == sql.ErrNoRows {
		return nil, fmt.Errorf("workflow not found: %s", workflowID)
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get workflow: %w", err)
	}

	workflow.ID = workflowID
	if metadataJSON != "" {
		json.Unmarshal([]byte(metadataJSON), &workflow.Metadata)
	}

	// Get current agent assignment
	var agentID string
	_ = cb.coordDB.QueryRowContext(ctx, `
		SELECT agent_id FROM agent_assignments 
		WHERE workflow_id = ? AND status IN ('assigned', 'started')
		ORDER BY assigned_at DESC LIMIT 1
	`, workflowID).Scan(&agentID)
	workflow.AgentID = agentID

	return &workflow, nil
}

// UpdateWorkflowStatus updates the status of a workflow
func (cb *CoordinationBridge) UpdateWorkflowStatus(ctx context.Context, workflowID, status string) error {
	result, err := cb.coordDB.ExecContext(ctx, `
		UPDATE workflow_mappings 
		SET status = ?, updated_at = ?
		WHERE tempolite_workflow_id = ?
	`, status, time.Now(), workflowID)

	if err != nil {
		return fmt.Errorf("failed to update workflow status: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to check rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("workflow not found: %s", workflowID)
	}

	// Update corresponding beads issue (best-effort -- log but don't fail the workflow update)
	var beadsIssueID string
	if err := cb.coordDB.QueryRowContext(ctx, `
		SELECT beads_issue_id FROM workflow_mappings WHERE tempolite_workflow_id = ?
	`, workflowID).Scan(&beadsIssueID); err == nil && beadsIssueID != "" {
		issueStatus := map[string]string{
			models.WorkflowStatusActive:    "in_progress",
			models.WorkflowStatusCompleted: "closed",
			models.WorkflowStatusFailed:    "blocked",
			models.WorkflowStatusPaused:    "blocked",
			models.WorkflowStatusCancelled: "cancelled",
		}[status]

		if issueStatus != "" {
			if err := cb.beadsClient.UpdateIssue(ctx, beadsIssueID, map[string]interface{}{
				"status": issueStatus,
			}); err != nil {
				// Best-effort: log but don't fail the status update
				fmt.Printf("warning: failed to sync beads issue %s: %v\n", beadsIssueID, err)
			}
		}
	}

	cb.eventBus.Publish(Event{
		Type:       "workflow:status_changed",
		WorkflowID: workflowID,
		Data: map[string]interface{}{
			"new_status": status,
		},
		Timestamp: time.Now(),
	})

	return nil
}

// CancelWorkflow cancels a workflow
func (cb *CoordinationBridge) CancelWorkflow(ctx context.Context, workflowID, reason string) error {
	if err := cb.UpdateWorkflowStatus(ctx, workflowID, models.WorkflowStatusCancelled); err != nil {
		return err
	}

	// Cancel agent assignments
	_, err := cb.coordDB.ExecContext(ctx, `
		UPDATE agent_assignments 
		SET status = 'cancelled', completed_at = ?
		WHERE workflow_id = ? AND status IN ('assigned', 'started')
	`, time.Now(), workflowID)

	if err != nil {
		return err
	}

	// Best-effort: add cancel comment to beads issue
	var beadsIssueID string
	if err := cb.coordDB.QueryRowContext(ctx, `
		SELECT beads_issue_id FROM workflow_mappings WHERE tempolite_workflow_id = ?
	`, workflowID).Scan(&beadsIssueID); err == nil && beadsIssueID != "" {
		_ = cb.beadsClient.AddComment(ctx, beadsIssueID, fmt.Sprintf("Workflow cancelled: %s", reason))
	}

	return nil
}

// RegisterAgent registers a new agent
func (cb *CoordinationBridge) RegisterAgent(ctx context.Context, agent *models.Agent) error {
	configJSON, _ := json.Marshal(agent.Configuration)
	capsJSON, _ := json.Marshal(agent.Capabilities)
	perfJSON, _ := json.Marshal(agent.PerformanceStats)

	_, err := cb.coordDB.ExecContext(ctx, `
		INSERT INTO agent_configurations 
		(id, agent_type, agent_id, configuration, capabilities, max_workload, current_workload, status, last_heartbeat, performance_metrics, created_at, updated_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(agent_type, agent_id) DO UPDATE SET
		configuration = excluded.configuration,
		capabilities = excluded.capabilities,
		max_workload = excluded.max_workload,
		status = excluded.status,
		last_heartbeat = excluded.last_heartbeat,
		performance_metrics = excluded.performance_metrics,
		updated_at = excluded.updated_at
	`,
		uuid.New().String(),
		agent.Type,
		agent.ID,
		configJSON,
		capsJSON,
		agent.MaxWorkload,
		agent.CurrentWorkload,
		agent.Status,
		agent.LastHeartbeat,
		perfJSON,
		time.Now(),
		time.Now(),
	)

	if err != nil {
		return fmt.Errorf("failed to register agent: %w", err)
	}

	return nil
}

// UnregisterAgent unregisters an agent
func (cb *CoordinationBridge) UnregisterAgent(ctx context.Context, agentID string) error {
	_, err := cb.coordDB.ExecContext(ctx, `
		UPDATE agent_configurations 
		SET status = 'inactive', updated_at = ?
		WHERE agent_id = ?
	`, time.Now(), agentID)

	return err
}

// GetAgentStatus retrieves agent status
func (cb *CoordinationBridge) GetAgentStatus(ctx context.Context, agentID string) (*models.Agent, error) {
	var agent models.Agent
	var configJSON, capsJSON, perfJSON string

	err := cb.coordDB.QueryRowContext(ctx, `
		SELECT agent_type, agent_id, configuration, capabilities, max_workload, current_workload, status, last_heartbeat, performance_metrics
		FROM agent_configurations WHERE agent_id = ?
	`, agentID).Scan(
		&agent.Type, &agent.ID, &configJSON, &capsJSON, &agent.MaxWorkload,
		&agent.CurrentWorkload, &agent.Status, &agent.LastHeartbeat, &perfJSON,
	)

	if err == sql.ErrNoRows {
		return nil, fmt.Errorf("agent not found: %s", agentID)
	}
	if err != nil {
		return nil, err
	}

	json.Unmarshal([]byte(configJSON), &agent.Configuration)
	json.Unmarshal([]byte(capsJSON), &agent.Capabilities)
	json.Unmarshal([]byte(perfJSON), &agent.PerformanceStats)

	return &agent, nil
}

// AssignWorkflow assigns a workflow to an agent
func (cb *CoordinationBridge) AssignWorkflow(ctx context.Context, workflowID, agentID string) error {
	// Get agent type
	var agentType string
	err := cb.coordDB.QueryRowContext(ctx, `
		SELECT agent_type FROM agent_configurations WHERE agent_id = ?
	`, agentID).Scan(&agentType)

	if err != nil {
		return fmt.Errorf("agent not found: %s", agentID)
	}

	// Create assignment
	_, err = cb.coordDB.ExecContext(ctx, `
		INSERT INTO agent_assignments (workflow_id, agent_type, agent_id, status, assigned_at)
		VALUES (?, ?, ?, 'assigned', ?)
	`, workflowID, agentType, agentID, time.Now())

	if err != nil {
		return fmt.Errorf("failed to assign workflow: %w", err)
	}

	// Update agent workload
	if _, err := cb.coordDB.ExecContext(ctx, `
		UPDATE agent_configurations 
		SET current_workload = current_workload + 1
		WHERE agent_id = ?
	`, agentID); err != nil {
		return fmt.Errorf("failed to update agent workload: %w", err)
	}

	return nil
}

// StoreResults stores workflow execution results
func (cb *CoordinationBridge) StoreResults(ctx context.Context, workflowID string, results *models.Result) error {
	dataJSON, _ := json.Marshal(results.Data)
	artifactsJSON, _ := json.Marshal(results.Artifacts)

	_, err := cb.coordDB.ExecContext(ctx, `
		INSERT INTO workflow_results 
		(workflow_id, agent_type, result_type, result_data, confidence_score, quality_score, execution_time_ms, artifacts, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
	`,
		workflowID,
		results.AgentType,
		results.ResultType,
		dataJSON,
		results.ConfidenceScore,
		results.QualityScore,
		results.ExecutionTimeMs,
		artifactsJSON,
		time.Now(),
	)

	if err != nil {
		return fmt.Errorf("failed to store results: %w", err)
	}

	// Best-effort: sync comment to beads issue
	var beadsIssueID string
	if err := cb.coordDB.QueryRowContext(ctx, `
		SELECT beads_issue_id FROM workflow_mappings WHERE tempolite_workflow_id = ?
	`, workflowID).Scan(&beadsIssueID); err == nil && beadsIssueID != "" {
		_ = cb.beadsClient.AddComment(ctx, beadsIssueID,
			fmt.Sprintf("Results stored: %s (confidence: %.2f)", results.ResultType, results.ConfidenceScore))
	}

	return nil
}

// GetResults retrieves workflow results
func (cb *CoordinationBridge) GetResults(ctx context.Context, workflowID string) ([]*models.Result, error) {
	rows, err := cb.coordDB.QueryContext(ctx, `
		SELECT workflow_id, agent_type, result_type, result_data, confidence_score, quality_score, execution_time_ms, artifacts, created_at
		FROM workflow_results WHERE workflow_id = ? ORDER BY created_at DESC
	`, workflowID)

	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var results []*models.Result
	for rows.Next() {
		var r models.Result
		var dataJSON, artifactsJSON string
		if err := rows.Scan(&r.WorkflowID, &r.AgentType, &r.ResultType, &dataJSON,
			&r.ConfidenceScore, &r.QualityScore, &r.ExecutionTimeMs, &artifactsJSON, &r.CreatedAt); err != nil {
			return nil, err
		}
		json.Unmarshal([]byte(dataJSON), &r.Data)
		json.Unmarshal([]byte(artifactsJSON), &r.Artifacts)
		results = append(results, &r)
	}

	return results, rows.Err()
}

// GetWorkflowAnalytics retrieves workflow analytics
func (cb *CoordinationBridge) GetWorkflowAnalytics(ctx context.Context, filters *models.AnalyticsFilters) (*models.Analytics, error) {
	query := `
		SELECT 
			COUNT(*) as total,
			SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
			SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed
		FROM workflow_mappings WHERE 1=1
	`
	var args []interface{}

	if filters != nil {
		if filters.WorkflowType != "" {
			query += " AND workflow_type = ?"
			args = append(args, filters.WorkflowType)
		}
		if filters.StartDate != nil {
			query += " AND created_at >= ?"
			args = append(args, *filters.StartDate)
		}
		if filters.EndDate != nil {
			query += " AND created_at <= ?"
			args = append(args, *filters.EndDate)
		}
	}

	var total, completed, failed int
	err := cb.coordDB.QueryRowContext(ctx, query, args...).Scan(&total, &completed, &failed)
	if err != nil {
		return nil, err
	}

	successRate := 0.0
	if total > 0 {
		successRate = float64(completed) / float64(total) * 100
	}

	return &models.Analytics{
		Period:             "custom",
		TotalWorkflows:     total,
		CompletedWorkflows: completed,
		FailedWorkflows:    failed,
		SuccessRate:        successRate,
	}, nil
}

// GetPerformanceMetrics retrieves performance metrics
func (cb *CoordinationBridge) GetPerformanceMetrics(ctx context.Context, period time.Duration) (map[string]interface{}, error) {
	since := time.Now().Add(-period)

	var avgDuration, avgConfidence float64
	err := cb.coordDB.QueryRowContext(ctx, `
		SELECT AVG(execution_time_ms), AVG(confidence_score)
		FROM workflow_results WHERE created_at >= ?
	`, since).Scan(&avgDuration, &avgConfidence)

	if err != nil {
		return nil, err
	}

	return map[string]interface{}{
		"avg_execution_time_ms": avgDuration,
		"avg_confidence_score":  avgConfidence,
		"period":                period.String(),
	}, nil
}

// ExecuteViaTempolite runs a workflow function through the tempolite engine
// and records the entity mapping. Returns the Future so the caller can wait
// on results.
func (cb *CoordinationBridge) ExecuteViaTempolite(workflowID string, workflowFunc interface{}, args ...interface{}) (tempolite.Future, error) {
	future, err := cb.tempolite.ExecuteWorkflow(workflowFunc, nil, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to execute workflow via tempolite: %w", err)
	}

	// Wait for the entity ID to be assigned, then persist the mapping.
	go func() {
		_ = future.WaitForIDs(context.Background())
		if future.HasEntity() {
			entityID := future.EntityID().(tempolite.WorkflowEntityID)
			cb.MapWorkflowEntity(workflowID, entityID)
			cb.persistEntityMapping(workflowID, entityID)
		}
	}()

	return future, nil
}

// SendHandoffSignal delivers a handoff signal through the tempolite engine.
func (cb *CoordinationBridge) SendHandoffSignal(workflowID string, fromAgent, toAgent string) error {
	entityID, ok := cb.GetTempoliteEntityID(workflowID)
	if !ok {
		return fmt.Errorf("no tempolite entity mapping for workflow %s", workflowID)
	}
	return cb.tempolite.PublishSignal(entityID, "agent_handoff", map[string]interface{}{
		"from_agent": fromAgent,
		"to_agent":   toAgent,
		"timestamp":  time.Now(),
	})
}

// persistEntityMapping writes the tempolite entity ID to the coordination DB.
func (cb *CoordinationBridge) persistEntityMapping(workflowID string, entityID tempolite.WorkflowEntityID) {
	_, _ = cb.coordDB.Exec(`
		UPDATE workflow_mappings SET tempolite_entity_id = ? WHERE tempolite_workflow_id = ?
	`, int(entityID), workflowID)
}

// loadEntityMappings restores the in-memory entity map from the coordination DB.
// Call this on startup after opening the database.
func (cb *CoordinationBridge) loadEntityMappings() error {
	rows, err := cb.coordDB.Query(`
		SELECT tempolite_workflow_id, tempolite_entity_id 
		FROM workflow_mappings WHERE tempolite_entity_id IS NOT NULL
	`)
	if err != nil {
		return err
	}
	defer rows.Close()

	for rows.Next() {
		var wfID string
		var entityID int
		if err := rows.Scan(&wfID, &entityID); err != nil {
			continue
		}
		cb.entityMap[wfID] = tempolite.WorkflowEntityID(entityID)
	}
	return rows.Err()
}

// TempoliteEngine returns the underlying tempolite engine for direct access.
func (cb *CoordinationBridge) TempoliteEngine() *tempolite.Engine {
	return cb.tempolite
}

// CoordinateHandoff coordinates agent handoff
func (cb *CoordinationBridge) CoordinateHandoff(ctx context.Context, workflowID string, fromAgent, toAgent string) error {
	// Create handoff signal
	signal := map[string]interface{}{
		"from_agent": fromAgent,
		"to_agent":   toAgent,
		"timestamp":  time.Now(),
	}

	// Deliver via tempolite if the workflow has an entity mapping.
	if err := cb.SendHandoffSignal(workflowID, fromAgent, toAgent); err != nil {
		// Best-effort: log but don't fail the handoff coordination.
		fmt.Printf("warning: tempolite signal delivery failed for %s: %v\n", workflowID, err)
	}
	_ = signal

	// Update current assignment
	_, err := cb.coordDB.ExecContext(ctx, `
		UPDATE agent_assignments 
		SET status = 'completed', completed_at = ?, handoff_to = ?
		WHERE workflow_id = ? AND agent_id = ? AND status = 'started'
	`, time.Now(), toAgent, workflowID, fromAgent)

	if err != nil {
		return err
	}

	// Create new assignment
	var agentType string
	_ = cb.coordDB.QueryRowContext(ctx, `
		SELECT agent_type FROM agent_configurations WHERE agent_id = ?
	`, toAgent).Scan(&agentType)

	_, err = cb.coordDB.ExecContext(ctx, `
		INSERT INTO agent_assignments (workflow_id, agent_type, agent_id, status, assigned_at, handoff_from)
		VALUES (?, ?, ?, 'assigned', ?, ?)
	`, workflowID, agentType, toAgent, time.Now(), fromAgent)

	if err != nil {
		return err
	}

	// Add comment to beads issue
	var beadsIssueID string
	_ = cb.coordDB.QueryRowContext(ctx, `
		SELECT beads_issue_id FROM workflow_mappings WHERE tempolite_workflow_id = ?
	`, workflowID).Scan(&beadsIssueID)

	if beadsIssueID != "" {
		cb.beadsClient.AddComment(ctx, beadsIssueID,
			fmt.Sprintf("Handoff from %s to %s", fromAgent, toAgent))
	}

	return nil
}

// RecoveryResult holds the outcome of a crash recovery
type RecoveryResult struct {
	WorkflowsFound    int      `json:"workflows_found"`
	WorkflowsRestored int      `json:"workflows_restored"`
	WorkflowsFailed   []string `json:"workflows_failed,omitempty"`
	AgentsReset       int64    `json:"agents_reset"`
	Errors            []string `json:"errors,omitempty"`
}

// RecoverFromCrash recovers the system after a crash
func (cb *CoordinationBridge) RecoverFromCrash(ctx context.Context) (*RecoveryResult, error) {
	result := &RecoveryResult{}

	// 1. Check coordination database health
	if err := cb.coordDB.PingContext(ctx); err != nil {
		return nil, fmt.Errorf("coordination database unhealthy: %w", err)
	}

	// 2. Find incomplete workflows
	rows, err := cb.coordDB.QueryContext(ctx, `
		SELECT tempolite_workflow_id FROM workflow_mappings 
		WHERE status IN ('active', 'paused')
	`)
	if err != nil {
		return nil, fmt.Errorf("failed to find incomplete workflows: %w", err)
	}
	defer rows.Close()

	var workflowIDs []string
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			result.Errors = append(result.Errors, fmt.Sprintf("scan error: %v", err))
			continue
		}
		workflowIDs = append(workflowIDs, id)
	}
	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating incomplete workflows: %w", err)
	}
	result.WorkflowsFound = len(workflowIDs)

	// 3. Restore each workflow via tempolite Resume
	for _, workflowID := range workflowIDs {
		entityID, ok := cb.GetTempoliteEntityID(workflowID)
		if !ok {
			result.WorkflowsFailed = append(result.WorkflowsFailed, workflowID)
			result.Errors = append(result.Errors, fmt.Sprintf("restore %s: no tempolite entity mapping", workflowID))
			continue
		}
		if _, err := cb.tempolite.ResumeWorkflow(entityID); err != nil {
			result.WorkflowsFailed = append(result.WorkflowsFailed, workflowID)
			result.Errors = append(result.Errors, fmt.Sprintf("restore %s: %v", workflowID, err))
			continue
		}
		result.WorkflowsRestored++

		// Publish recovery event
		cb.eventBus.Publish(Event{
			Type:       "workflow:recovered",
			WorkflowID: workflowID,
			Data:       map[string]interface{}{"source": "crash_recovery"},
			Timestamp:  time.Now(),
		})
	}

	// 4. Resume agent assignments
	res, err := cb.coordDB.ExecContext(ctx, `
		UPDATE agent_assignments 
		SET status = 'assigned'
		WHERE status = 'started'
	`)
	if err != nil {
		result.Errors = append(result.Errors, fmt.Sprintf("agent reset: %v", err))
	} else {
		result.AgentsReset, _ = res.RowsAffected()
	}

	// 5. Reconcile beads issue state for restored workflows
	for _, workflowID := range workflowIDs {
		var beadsIssueID string
		if err := cb.coordDB.QueryRowContext(ctx, `
			SELECT beads_issue_id FROM workflow_mappings WHERE tempolite_workflow_id = ?
		`, workflowID).Scan(&beadsIssueID); err != nil {
			continue
		}
		if err := cb.beadsClient.UpdateIssue(ctx, beadsIssueID, map[string]interface{}{
			"status": "in_progress",
		}); err != nil {
			result.Errors = append(result.Errors, fmt.Sprintf("beads sync %s: %v", beadsIssueID, err))
		}
	}

	return result, nil
}

// Helper functions
func generateWorkflowID(workflowType string) string {
	return fmt.Sprintf("wf-%s-%s", workflowType, uuid.New().String()[:8])
}

func generateAgentID(agentType string) string {
	return fmt.Sprintf("%s-agent-%s", agentType, uuid.New().String()[:8])
}
