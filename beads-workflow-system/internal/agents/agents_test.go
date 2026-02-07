package agents

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/your-org/beads-workflow-system/pkg/models"
)

func TestRegistryRegisterAndGet(t *testing.T) {
	registry := NewRegistry()
	registry.Register("test", func(id string) Agent {
		return &ResearchAgent{BaseAgent: BaseAgent{ID: id, Type: "test"}}
	})

	agent, err := registry.Get("test", "agent-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if agent.GetID() != "agent-1" {
		t.Errorf("got ID %q, want %q", agent.GetID(), "agent-1")
	}
}

func TestRegistryGetUnknownType(t *testing.T) {
	registry := NewRegistry()
	_, err := registry.Get("nonexistent", "id")
	if err == nil {
		t.Fatal("expected error for unknown agent type")
	}
}

func TestRegistryListTypes(t *testing.T) {
	registry := NewRegistry()
	registry.Register("a", func(id string) Agent { return nil })
	registry.Register("b", func(id string) Agent { return nil })

	types := registry.ListTypes()
	if len(types) != 2 {
		t.Fatalf("got %d types, want 2", len(types))
	}
}

func TestDefaultRegistryHasAllAgents(t *testing.T) {
	types := []string{models.AgentTypeResearch, models.AgentTypePOC, models.AgentTypeDocumentation, models.AgentTypeValidation}

	for _, agentType := range types {
		agent, err := DefaultRegistry.Get(agentType, "test-agent")
		if err != nil {
			t.Errorf("DefaultRegistry missing agent type %q: %v", agentType, err)
			continue
		}
		if agent.GetType() != agentType {
			t.Errorf("agent type = %q, want %q", agent.GetType(), agentType)
		}
	}
}

func TestResearchAgentExecute(t *testing.T) {
	agent := NewResearchAgent("test-research")

	workflow := &models.Workflow{
		ID:   "wf-test-1",
		Type: models.AgentTypeResearch,
		Variables: map[string]interface{}{
			"query": "test query",
			"focus": "performance",
		},
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.AgentType != models.AgentTypeResearch {
		t.Errorf("result agent type = %q, want %q", result.AgentType, models.AgentTypeResearch)
	}
	if result.ConfidenceScore <= 0 || result.ConfidenceScore > 1.0 {
		t.Errorf("confidence score out of range: %f", result.ConfidenceScore)
	}
	if result.QualityScore <= 0 || result.QualityScore > 10.0 {
		t.Errorf("quality score out of range: %f", result.QualityScore)
	}
	if len(result.Artifacts) == 0 {
		t.Error("expected at least one artifact")
	}
}

func TestResearchAgentValidateWrongType(t *testing.T) {
	agent := NewResearchAgent("test")

	workflow := &models.Workflow{
		ID:   "wf-test",
		Type: models.AgentTypePOC, // Wrong type
	}

	if err := agent.Validate(workflow); err == nil {
		t.Error("expected validation error for wrong workflow type")
	}
}

func TestPOCAgentExecute(t *testing.T) {
	agent := NewPOCAgent("test-poc")

	workflow := &models.Workflow{
		ID:   "wf-test-poc",
		Type: models.AgentTypePOC,
		Variables: map[string]interface{}{
			"language":  "rust",
			"framework": "tokio",
		},
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ResultType != models.ResultTypePOCResults {
		t.Errorf("result type = %q, want %q", result.ResultType, models.ResultTypePOCResults)
	}
	if len(result.Artifacts) == 0 {
		t.Error("expected at least one artifact")
	}
}

func TestDocumentationAgentExecute(t *testing.T) {
	agent := NewDocumentationAgent("test-doc")

	workflow := &models.Workflow{
		ID:   "wf-test-doc",
		Type: models.AgentTypeDocumentation,
		Variables: map[string]interface{}{
			"project_name": "TestProject",
		},
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ConfidenceScore <= 0 {
		t.Error("expected positive confidence score")
	}
	if result.ResultType != models.ResultTypeDocumentation {
		t.Errorf("result type = %q, want %q", result.ResultType, models.ResultTypeDocumentation)
	}
}

func TestValidationAgentExecute(t *testing.T) {
	agent := NewValidationAgent("test-val")

	workflow := &models.Workflow{
		ID:   "wf-test-val",
		Type: models.AgentTypeValidation,
		Variables: map[string]interface{}{
			"checklist_id": "default",
		},
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if result.ConfidenceScore < 0 || result.ConfidenceScore > 1.0 {
		t.Errorf("confidence out of range: %f", result.ConfidenceScore)
	}
	if result.ResultType != models.ResultTypeValidation {
		t.Errorf("result type = %q, want %q", result.ResultType, models.ResultTypeValidation)
	}
}

func TestExecuteStepRetry(t *testing.T) {
	callCount := 0
	step := Step{
		Name:       "flaky_step",
		RetryCount: 2,
		Timeout:    5 * time.Second,
		Execute: func(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
			callCount++
			if callCount < 3 {
				return nil, fmt.Errorf("transient error")
			}
			return map[string]interface{}{"ok": true}, nil
		},
	}

	result, err := ExecuteStep(context.Background(), step, nil)
	if err != nil {
		t.Fatalf("expected step to succeed after retries: %v", err)
	}
	if result["ok"] != true {
		t.Error("expected ok=true")
	}
	if callCount != 3 {
		t.Errorf("expected 3 calls, got %d", callCount)
	}
}

func TestExecuteStepAllRetriesFail(t *testing.T) {
	step := Step{
		Name:       "always_fails",
		RetryCount: 1,
		Execute: func(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
			return nil, fmt.Errorf("permanent error")
		},
	}

	_, err := ExecuteStep(context.Background(), step, nil)
	if err == nil {
		t.Fatal("expected error after all retries exhausted")
	}
}

func TestTemplateManagerListAndGet(t *testing.T) {
	tm := NewTemplateManager()

	all := tm.ListTemplates("")
	if len(all) != 8 {
		t.Errorf("expected 8 templates, got %d", len(all))
	}

	research := tm.ListTemplates(models.AgentTypeResearch)
	if len(research) != 2 {
		t.Errorf("expected 2 research templates, got %d", len(research))
	}

	tmpl, err := tm.GetTemplate("poc-full")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if tmpl.AgentType != models.AgentTypePOC {
		t.Errorf("template agent type = %q, want %q", tmpl.AgentType, models.AgentTypePOC)
	}

	_, err = tm.GetTemplate("nonexistent")
	if err == nil {
		t.Fatal("expected error for nonexistent template")
	}
}

func TestTemplateManagerApply(t *testing.T) {
	tm := NewTemplateManager()

	req, err := tm.ApplyTemplate("research-basic", map[string]interface{}{
		"query": "test",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if req.WorkflowType != models.AgentTypeResearch {
		t.Errorf("workflow type = %q, want %q", req.WorkflowType, models.AgentTypeResearch)
	}
	if req.Variables["query"] != "test" {
		t.Error("user variable should override template default")
	}
	// Template default "focus" should be present
	if req.Variables["focus"] == nil {
		t.Error("template default variable 'focus' should be included")
	}
}