package agents

import (
	"context"
	"fmt"
	"time"

	"github.com/your-org/beads-workflow-system/pkg/models"
)

// Agent is the interface that all workflow agents must implement
type Agent interface {
	// GetType returns the agent type
	GetType() string
	
	// GetID returns the unique agent ID
	GetID() string
	
	// GetCapabilities returns what this agent can do
	GetCapabilities() []string
	
	// Execute runs the agent on a workflow
	Execute(ctx context.Context, workflow *models.Workflow) (*models.Result, error)
	
	// GetSteps returns the sequence of steps this agent performs
	GetSteps() []Step
	
	// Validate checks if the agent can handle the given workflow
	Validate(workflow *models.Workflow) error
}

// Step represents a single step in an agent's workflow
type Step struct {
	Name        string
	Description string
	Execute     func(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error)
	Timeout     time.Duration
	RetryCount  int
}

// BaseAgent provides common functionality for all agents
type BaseAgent struct {
	ID           string
	Type         string
	Capabilities []string
	Steps        []Step
	MaxRetries   int
	Timeout      time.Duration
}

// GetType returns the agent type
func (ba *BaseAgent) GetType() string {
	return ba.Type
}

// GetID returns the agent ID
func (ba *BaseAgent) GetID() string {
	return ba.ID
}

// GetCapabilities returns agent capabilities
func (ba *BaseAgent) GetCapabilities() []string {
	return ba.Capabilities
}

// GetSteps returns the agent's steps
func (ba *BaseAgent) GetSteps() []Step {
	return ba.Steps
}

// Validate checks if the workflow can be handled
func (ba *BaseAgent) Validate(workflow *models.Workflow) error {
	if workflow == nil {
		return fmt.Errorf("workflow cannot be nil")
	}
	
	if workflow.Type != ba.Type {
		return fmt.Errorf("agent type %s cannot handle workflow type %s", ba.Type, workflow.Type)
	}
	
	return nil
}

// ActivityContext provides context for activity execution
type ActivityContext struct {
	context.Context
	WorkflowID   string
	StepNumber   int
	StepName     string
	Input        map[string]interface{}
	StartTime    time.Time
	Results      map[string]interface{}
}

// NewActivityContext creates a new activity context
func NewActivityContext(ctx context.Context, workflowID string, stepNumber int, stepName string) *ActivityContext {
	return &ActivityContext{
		Context:    ctx,
		WorkflowID: workflowID,
		StepNumber: stepNumber,
		StepName:   stepName,
		Input:      make(map[string]interface{}),
		StartTime:  time.Now(),
		Results:    make(map[string]interface{}),
	}
}

// AgentFactory creates agents
type AgentFactory func(agentID string) Agent

// Registry holds all available agents
type Registry struct {
	agents map[string]AgentFactory
}

// NewRegistry creates a new agent registry
func NewRegistry() *Registry {
	return &Registry{
		agents: make(map[string]AgentFactory),
	}
}

// Register adds an agent factory to the registry
func (r *Registry) Register(agentType string, factory AgentFactory) {
	r.agents[agentType] = factory
}

// Get creates an agent of the specified type
func (r *Registry) Get(agentType, agentID string) (Agent, error) {
	factory, exists := r.agents[agentType]
	if !exists {
		return nil, fmt.Errorf("unknown agent type: %s", agentType)
	}
	
	return factory(agentID), nil
}

// ListTypes returns all registered agent types
func (r *Registry) ListTypes() []string {
	types := make([]string, 0, len(r.agents))
	for t := range r.agents {
		types = append(types, t)
	}
	return types
}

// Executor handles agent execution
type Executor struct {
	registry *Registry
}

// NewExecutor creates a new agent executor
func NewExecutor(registry *Registry) *Executor {
	return &Executor{
		registry: registry,
	}
}

// ExecuteWorkflow executes a workflow using the appropriate agent
func (e *Executor) ExecuteWorkflow(ctx context.Context, workflow *models.Workflow) (*models.Result, error) {
	// Get the agent for this workflow type
	agent, err := e.registry.Get(workflow.Type, workflow.AgentID)
	if err != nil {
		return nil, fmt.Errorf("failed to get agent: %w", err)
	}
	
	// Validate the workflow
	if err := agent.Validate(workflow); err != nil {
		return nil, fmt.Errorf("workflow validation failed: %w", err)
	}
	
	// Execute the agent
	return agent.Execute(ctx, workflow)
}

// ExecuteStep executes a single step with retry logic
func ExecuteStep(ctx context.Context, step Step, input map[string]interface{}) (map[string]interface{}, error) {
	var result map[string]interface{}
	var err error
	
	for attempt := 0; attempt <= step.RetryCount; attempt++ {
		if attempt > 0 {
			// Wait before retry
			time.Sleep(time.Second * time.Duration(attempt))
		}
		
		// Create timeout context if specified
		stepCtx := ctx
		if step.Timeout > 0 {
			var cancel context.CancelFunc
			stepCtx, cancel = context.WithTimeout(ctx, step.Timeout)
			defer cancel()
		}
		
		result, err = step.Execute(stepCtx, input)
		if err == nil {
			return result, nil
		}
		
		// Check if context was cancelled
		if ctx.Err() != nil {
			return nil, ctx.Err()
		}
	}
	
	return nil, fmt.Errorf("step failed after %d attempts: %w", step.RetryCount+1, err)
}

// DefaultRegistry is the global agent registry
var DefaultRegistry = NewRegistry()

// Register registers an agent factory with the default registry
func Register(agentType string, factory AgentFactory) {
	DefaultRegistry.Register(agentType, factory)
}