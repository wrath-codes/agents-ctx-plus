package tempolite

import (
	"context"
	"fmt"
	"time"

	tp "github.com/davidroman0O/tempolite"
)

// Engine provides workflow execution capabilities backed by the real tempolite library.
type Engine struct {
	tp *tp.Tempolite
}

// NewEngine creates a new Tempolite engine with an in-memory database.
func NewEngine() (*Engine, error) {
	ctx := context.Background()
	db := tp.NewMemoryDatabase()

	instance, err := tp.New(ctx, db)
	if err != nil {
		return nil, fmt.Errorf("failed to create tempolite instance: %w", err)
	}
	return &Engine{tp: instance}, nil
}

// NewEngineWithOptions creates a new Tempolite engine with custom options.
func NewEngineWithOptions(opts ...tp.TempoliteOption) (*Engine, error) {
	ctx := context.Background()
	db := tp.NewMemoryDatabase()

	instance, err := tp.New(ctx, db, opts...)
	if err != nil {
		return nil, fmt.Errorf("failed to create tempolite instance: %w", err)
	}
	return &Engine{tp: instance}, nil
}

// Tempolite returns the underlying tempolite instance for direct access.
func (e *Engine) Tempolite() *tp.Tempolite {
	return e.tp
}

// RegisterWorkflow pre-registers a workflow function for pause/resume support.
func (e *Engine) RegisterWorkflow(workflowFunc interface{}) error {
	return e.tp.RegisterWorkflow(workflowFunc)
}

// ExecuteWorkflow executes a workflow on the default queue.
// workflowFunc is the workflow function, options can be nil, args are passed to the function.
// Returns a Future that can be used to get the result.
func (e *Engine) ExecuteWorkflow(workflowFunc interface{}, options *tp.WorkflowOptions, args ...interface{}) (tp.Future, error) {
	return e.tp.ExecuteDefault(workflowFunc, options, args...)
}

// ExecuteWorkflowOnQueue executes a workflow on a named queue.
func (e *Engine) ExecuteWorkflowOnQueue(queueName string, workflowFunc interface{}, options *tp.WorkflowOptions, args ...interface{}) (tp.Future, error) {
	return e.tp.Execute(queueName, workflowFunc, options, args...)
}

// GetWorkflow returns a workflow entity by ID.
func (e *Engine) GetWorkflow(id tp.WorkflowEntityID) (*tp.WorkflowEntity, error) {
	return e.tp.GetWorkflow(id)
}

// GetWorkflowResult returns a Future for a completed workflow's results.
func (e *Engine) GetWorkflowResult(id tp.WorkflowEntityID) (tp.Future, error) {
	return e.tp.Get(id)
}

// PublishSignal sends a signal to a running workflow.
func (e *Engine) PublishSignal(workflowID tp.WorkflowEntityID, signalName string, value interface{}) error {
	return e.tp.PublishSignal(workflowID, signalName, value)
}

// PauseWorkflow pauses a running workflow at the next context call.
func (e *Engine) PauseWorkflow(queueName string, id tp.WorkflowEntityID) error {
	return e.tp.Pause(queueName, id)
}

// ResumeWorkflow resumes a paused workflow.
func (e *Engine) ResumeWorkflow(id tp.WorkflowEntityID) (tp.Future, error) {
	return e.tp.Resume(id)
}

// CreateQueue creates a new named queue.
func (e *Engine) CreateQueue(config tp.QueueConfig) error {
	return e.tp.CreateQueue(config)
}

// ScaleQueue adjusts the number of concurrent pools for a queue.
func (e *Engine) ScaleQueue(queueName string, targetCount int) error {
	return e.tp.Scale(queueName, targetCount)
}

// CountQueue returns the count of workflows in a queue by status.
func (e *Engine) CountQueue(queueName string, status tp.EntityStatus) (int, error) {
	return e.tp.CountQueue(queueName, status)
}

// Wait blocks until all queues have drained.
func (e *Engine) Wait() error {
	return e.tp.Wait()
}

// Close closes the tempolite engine and releases resources.
func (e *Engine) Close() error {
	return e.tp.Close()
}

// --- Workflow helpers for the agent system ---

// ActivityResult holds the result of an activity execution.
type ActivityResult struct {
	Data      interface{}
	Error     error
	Duration  time.Duration
	StartedAt time.Time
	EndedAt   time.Time
}

// RunWorkflowSync executes a workflow and blocks until it completes.
// Returns the raw results or an error.
func (e *Engine) RunWorkflowSync(workflowFunc interface{}, args ...interface{}) ([]interface{}, error) {
	future, err := e.tp.ExecuteDefault(workflowFunc, nil, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to start workflow: %w", err)
	}
	results, err := future.GetResults()
	if err != nil {
		return nil, fmt.Errorf("workflow execution failed: %w", err)
	}
	return results, nil
}

// RunWorkflowSyncWithResult executes a workflow and deserializes the result into out.
func (e *Engine) RunWorkflowSyncWithResult(workflowFunc interface{}, out interface{}, args ...interface{}) error {
	future, err := e.tp.ExecuteDefault(workflowFunc, nil, args...)
	if err != nil {
		return fmt.Errorf("failed to start workflow: %w", err)
	}
	if err := future.Get(out); err != nil {
		return fmt.Errorf("workflow execution failed: %w", err)
	}
	return nil
}

// NewSagaBuilder creates a new saga definition builder.
func NewSagaBuilder() *tp.SagaDefinitionBuilder {
	return tp.NewSaga()
}

// Re-export key types so consumers don't need to import tempolite directly.
type (
	WorkflowContext  = tp.WorkflowContext
	ActivityContext  = tp.ActivityContext
	Future           = tp.Future
	WorkflowOptions  = tp.WorkflowOptions
	ActivityOptions  = tp.ActivityOptions
	RetryPolicy      = tp.RetryPolicy
	SagaDefinition   = tp.SagaDefinition
	QueueConfig      = tp.QueueConfig
	WorkflowEntityID = tp.WorkflowEntityID
	WorkflowEntity   = tp.WorkflowEntity
	EntityStatus     = tp.EntityStatus
)
