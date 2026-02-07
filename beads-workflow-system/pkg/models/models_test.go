package models

import (
	"testing"
)

func TestWorkflowStatusConstants(t *testing.T) {
	statuses := []string{
		WorkflowStatusActive,
		WorkflowStatusCompleted,
		WorkflowStatusFailed,
		WorkflowStatusPaused,
		WorkflowStatusCancelled,
	}

	seen := make(map[string]bool)
	for _, s := range statuses {
		if s == "" {
			t.Error("empty workflow status constant")
		}
		if seen[s] {
			t.Errorf("duplicate status constant: %s", s)
		}
		seen[s] = true
	}
}

func TestAgentTypeConstants(t *testing.T) {
	types := []string{
		AgentTypeResearch,
		AgentTypePOC,
		AgentTypeDocumentation,
		AgentTypeValidation,
		AgentTypeSupervisor,
	}

	seen := make(map[string]bool)
	for _, at := range types {
		if at == "" {
			t.Error("empty agent type constant")
		}
		if seen[at] {
			t.Errorf("duplicate agent type: %s", at)
		}
		seen[at] = true
	}
}

func TestResultTypeConstants(t *testing.T) {
	types := []string{
		ResultTypeFindings,
		ResultTypePOCResults,
		ResultTypeDocumentation,
		ResultTypeValidation,
		ResultTypePerformance,
	}

	seen := make(map[string]bool)
	for _, rt := range types {
		if rt == "" {
			t.Error("empty result type constant")
		}
		if seen[rt] {
			t.Errorf("duplicate result type: %s", rt)
		}
		seen[rt] = true
	}
}