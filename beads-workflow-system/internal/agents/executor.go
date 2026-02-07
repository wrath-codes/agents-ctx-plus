package agents

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"time"

	"github.com/your-org/beads-workflow-system/internal/bridge"
	"github.com/your-org/beads-workflow-system/internal/tempolite"
	"github.com/your-org/beads-workflow-system/pkg/models"
)

// WorkflowExecutor executes agent workflows
type WorkflowExecutor struct {
	registry *Registry
	bridge   *bridge.CoordinationBridge
	db       *sql.DB
}

// NewWorkflowExecutor creates a new workflow executor
func NewWorkflowExecutor(registry *Registry, coordBridge *bridge.CoordinationBridge, db *sql.DB) *WorkflowExecutor {
	return &WorkflowExecutor{
		registry: registry,
		bridge:   coordBridge,
		db:       db,
	}
}

// ExecuteWorkflow executes a workflow using the appropriate agent.
// If the tempolite engine is available, the agent is wrapped in a tempolite
// workflow function so that execution is durable and recoverable.
func (we *WorkflowExecutor) ExecuteWorkflow(ctx context.Context, workflowID string) error {
	// Get workflow details
	workflow, err := we.bridge.GetWorkflow(ctx, workflowID)
	if err != nil {
		return fmt.Errorf("failed to get workflow: %w", err)
	}

	// Update workflow status to started
	if err := we.updateWorkflowProgress(workflowID, 0, "starting"); err != nil {
		return err
	}

	// Get the appropriate agent
	agent, err := we.registry.Get(workflow.Type, workflow.AgentID)
	if err != nil {
		return fmt.Errorf("failed to get agent: %w", err)
	}

	// Record step start
	steps := agent.GetSteps()
	if err := we.recordStepStart(workflowID, 1, steps[0].Name); err != nil {
		return err
	}

	// Execute via tempolite if the engine is available.
	// We wrap the agent execution in a tempolite workflow function so
	// the engine can manage checkpointing and recovery.
	engine := we.bridge.TempoliteEngine()
	if engine != nil {
		return we.executeViaTempolite(ctx, engine, workflowID, workflow, agent)
	}

	// Fallback: direct execution without tempolite durability.
	return we.executeDirect(ctx, workflowID, workflow, agent)
}

// executeViaTempolite wraps the agent execution in a tempolite workflow.
func (we *WorkflowExecutor) executeViaTempolite(
	ctx context.Context,
	engine *tempolite.Engine,
	workflowID string,
	workflow *models.Workflow,
	agent Agent,
) error {
	// Define a tempolite-compatible workflow function that runs the agent.
	// NOTE: The agents are currently simulated -- this wiring is ready for
	// real agent implementations. See FUTURE TODO at bottom of file.
	agentWorkflow := func(wfCtx tempolite.WorkflowContext) (*models.Result, error) {
		return agent.Execute(ctx, workflow)
	}

	// Register and execute
	_ = engine.RegisterWorkflow(agentWorkflow)
	future, err := we.bridge.ExecuteViaTempolite(workflowID, agentWorkflow)
	if err != nil {
		we.updateWorkflowStatus(workflowID, models.WorkflowStatusFailed, err.Error())
		return fmt.Errorf("tempolite execution failed: %w", err)
	}

	// Block until the workflow completes
	var result *models.Result
	if err := future.Get(&result); err != nil {
		we.updateWorkflowStatus(workflowID, models.WorkflowStatusFailed, err.Error())
		return fmt.Errorf("workflow execution failed: %w", err)
	}

	// Store results
	if err := we.bridge.StoreResults(ctx, workflowID, result); err != nil {
		return fmt.Errorf("failed to store results: %w", err)
	}

	// Update workflow status to completed
	if err := we.bridge.UpdateWorkflowStatus(ctx, workflowID, models.WorkflowStatusCompleted); err != nil {
		return err
	}

	if err := we.updateWorkflowProgress(workflowID, 100, "completed"); err != nil {
		return err
	}

	return nil
}

// executeDirect runs the agent without tempolite durability (fallback path).
func (we *WorkflowExecutor) executeDirect(
	ctx context.Context,
	workflowID string,
	workflow *models.Workflow,
	agent Agent,
) error {
	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		we.updateWorkflowStatus(workflowID, models.WorkflowStatusFailed, err.Error())
		return fmt.Errorf("workflow execution failed: %w", err)
	}

	if err := we.bridge.StoreResults(ctx, workflowID, result); err != nil {
		return fmt.Errorf("failed to store results: %w", err)
	}

	if err := we.bridge.UpdateWorkflowStatus(ctx, workflowID, models.WorkflowStatusCompleted); err != nil {
		return err
	}

	if err := we.updateWorkflowProgress(workflowID, 100, "completed"); err != nil {
		return err
	}

	return nil
}

// FUTURE TODO: The 4 agent implementations (research, poc, documentation,
// validation) currently return simulated/hardcoded data. To make the system
// fully functional:
//
// 1. ResearchAgent: Should discover real libraries, fetch docs, run static analysis
// 2. POCAgent: Should generate real code, build, run tests, benchmark
// 3. DocumentationAgent: Should analyze real codebases, extract APIs, generate docs
// 4. ValidationAgent: Should run real code quality, security, performance checks
//
// Each agent's Execute() steps should become individual tempolite activities
// so that partial progress is checkpointed and recoverable.

// ExecuteWorkflowStep executes a single workflow step
func (we *WorkflowExecutor) ExecuteWorkflowStep(ctx context.Context, workflowID string, stepNumber int) error {
	// Get workflow
	workflow, err := we.bridge.GetWorkflow(ctx, workflowID)
	if err != nil {
		return err
	}

	// Get agent
	agent, err := we.registry.Get(workflow.Type, workflow.AgentID)
	if err != nil {
		return err
	}

	steps := agent.GetSteps()
	if stepNumber < 1 || stepNumber > len(steps) {
		return fmt.Errorf("invalid step number: %d", stepNumber)
	}

	step := steps[stepNumber-1]

	// Record step start
	if err := we.recordStepStart(workflowID, stepNumber, step.Name); err != nil {
		return err
	}

	// Get previous results
	results, err := we.getStepResults(workflowID)
	if err != nil {
		return err
	}

	// Prepare step input
	stepInput := map[string]interface{}{
		"workflow_id": workflowID,
		"step_number": stepNumber,
		"total_steps": len(steps),
		"variables":   workflow.Variables,
		"results":     results,
	}

	// Execute step
	stepResult, err := ExecuteStep(ctx, step, stepInput)
	if err != nil {
		we.recordStepFailure(workflowID, stepNumber, err.Error())
		return fmt.Errorf("step %d failed: %w", stepNumber, err)
	}

	// Record step completion
	if err := we.recordStepCompletion(workflowID, stepNumber, stepResult); err != nil {
		return err
	}

	// Update progress
	progress := float64(stepNumber) / float64(len(steps)) * 100
	if err := we.updateWorkflowProgress(workflowID, int(progress), step.Name); err != nil {
		return err
	}

	return nil
}

// GetWorkflowProgress retrieves the current progress of a workflow
func (we *WorkflowExecutor) GetWorkflowProgress(workflowID string) (*WorkflowProgress, error) {
	var progress WorkflowProgress

	err := we.db.QueryRow(`
		SELECT workflow_id, current_step, total_steps, progress_percent, status, updated_at
		FROM workflow_progress WHERE workflow_id = ?
	`, workflowID).Scan(
		&progress.WorkflowID,
		&progress.CurrentStep,
		&progress.TotalSteps,
		&progress.ProgressPercent,
		&progress.Status,
		&progress.UpdatedAt,
	)

	if err == sql.ErrNoRows {
		// Return default progress
		return &WorkflowProgress{
			WorkflowID:      workflowID,
			CurrentStep:     0,
			TotalSteps:      0,
			ProgressPercent: 0,
			Status:          "pending",
		}, nil
	}

	if err != nil {
		return nil, err
	}

	return &progress, nil
}

// WorkflowProgress tracks workflow execution progress
type WorkflowProgress struct {
	WorkflowID      string    `json:"workflow_id"`
	CurrentStep     int       `json:"current_step"`
	TotalSteps      int       `json:"total_steps"`
	ProgressPercent int       `json:"progress_percent"`
	Status          string    `json:"status"`
	CurrentStepName string    `json:"current_step_name,omitempty"`
	UpdatedAt       time.Time `json:"updated_at"`
}

// updateWorkflowProgress updates workflow progress in database
func (we *WorkflowExecutor) updateWorkflowProgress(workflowID string, progress int, stepName string) error {
	_, err := we.db.Exec(`
		INSERT INTO workflow_progress (workflow_id, progress_percent, current_step_name, status, updated_at)
		VALUES (?, ?, ?, 'active', ?)
		ON CONFLICT(workflow_id) DO UPDATE SET
			progress_percent = excluded.progress_percent,
			current_step_name = excluded.current_step_name,
			status = excluded.status,
			updated_at = excluded.updated_at
	`, workflowID, progress, stepName, time.Now())

	return err
}

// updateWorkflowStatus updates workflow status
func (we *WorkflowExecutor) updateWorkflowStatus(workflowID, status, message string) error {
	_, err := we.db.Exec(`
		UPDATE workflow_mappings 
		SET status = ?, updated_at = ?
		WHERE tempolite_workflow_id = ?
	`, status, time.Now(), workflowID)

	return err
}

// recordStepStart records the start of a workflow step
func (we *WorkflowExecutor) recordStepStart(workflowID string, stepNumber int, stepName string) error {
	_, err := we.db.Exec(`
		INSERT INTO workflow_performance (workflow_id, agent_type, step_name, step_number, start_time, success)
		VALUES (?, 'executor', ?, ?, ?, 0)
	`, workflowID, stepName, stepNumber, time.Now())

	return err
}

// recordStepCompletion records the completion of a workflow step
func (we *WorkflowExecutor) recordStepCompletion(workflowID string, stepNumber int, result map[string]interface{}) error {
	resultJSON, _ := json.Marshal(result)

	_, err := we.db.Exec(`
		UPDATE workflow_performance 
		SET end_time = ?, success = 1, resource_metadata = ?
		WHERE workflow_id = ? AND step_number = ?
	`, time.Now(), resultJSON, workflowID, stepNumber)

	return err
}

// recordStepFailure records a step failure
func (we *WorkflowExecutor) recordStepFailure(workflowID string, stepNumber int, errorMessage string) error {
	_, err := we.db.Exec(`
		UPDATE workflow_performance 
		SET end_time = ?, success = 0, error_message = ?
		WHERE workflow_id = ? AND step_number = ?
	`, time.Now(), errorMessage, workflowID, stepNumber)

	return err
}

// getStepResults retrieves accumulated results from previous steps
func (we *WorkflowExecutor) getStepResults(workflowID string) (map[string]interface{}, error) {
	rows, err := we.db.Query(`
		SELECT resource_metadata FROM workflow_performance
		WHERE workflow_id = ? AND success = 1
		ORDER BY step_number
	`, workflowID)

	if err != nil {
		return nil, err
	}
	defer rows.Close()

	results := make(map[string]interface{})

	for rows.Next() {
		var resultJSON string
		if err := rows.Scan(&resultJSON); err != nil {
			continue
		}

		var stepResult map[string]interface{}
		if err := json.Unmarshal([]byte(resultJSON), &stepResult); err != nil {
			continue
		}

		// Merge results
		for k, v := range stepResult {
			results[k] = v
		}
	}

	return results, rows.Err()
}

// CreateProgressTable creates the workflow progress tracking table
func CreateProgressTable(db *sql.DB) error {
	_, err := db.Exec(`
		CREATE TABLE IF NOT EXISTS workflow_progress (
			workflow_id TEXT PRIMARY KEY,
			current_step INTEGER DEFAULT 0,
			total_steps INTEGER DEFAULT 0,
			progress_percent INTEGER DEFAULT 0,
			status TEXT DEFAULT 'pending',
			current_step_name TEXT,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			
			FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
		)
	`)

	return err
}
