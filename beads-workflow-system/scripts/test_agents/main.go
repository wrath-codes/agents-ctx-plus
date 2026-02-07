package main

import (
	"context"
	"fmt"
	"time"

	"github.com/your-org/beads-workflow-system/internal/agents"
	"github.com/your-org/beads-workflow-system/pkg/models"
)

func main() {
	fmt.Println("=== Agent Workflow Test ===\n")
	
	// Test Research Agent
	fmt.Println("1. Testing Research Agent...")
	testResearchAgent()
	
	// Test POC Agent
	fmt.Println("\n2. Testing POC Agent...")
	testPOCAgent()
	
	// Test Documentation Agent
	fmt.Println("\n3. Testing Documentation Agent...")
	testDocumentationAgent()
	
	// Test Validation Agent
	fmt.Println("\n4. Testing Validation Agent...")
	testValidationAgent()
	
	// Test Templates
	fmt.Println("\n5. Testing Workflow Templates...")
	testTemplates()
	
	fmt.Println("\n=== All Tests Completed ===")
}

func testResearchAgent() {
	agent := agents.NewResearchAgent("test-research-agent")
	
	workflow := &models.Workflow{
		ID:        "wf-test-research",
		Type:      models.AgentTypeResearch,
		AgentID:   agent.GetID(),
		Variables: map[string]interface{}{
			"query": "async rust frameworks",
			"focus": "performance",
		},
	}
	
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	
	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		fmt.Printf("   ❌ Research agent failed: %v\n", err)
		return
	}
	
	fmt.Printf("   ✓ Research agent completed\n")
	fmt.Printf("     - Confidence: %.2f\n", result.ConfidenceScore)
	fmt.Printf("     - Quality: %.2f\n", result.QualityScore)
	fmt.Printf("     - Duration: %dms\n", result.ExecutionTimeMs)
	fmt.Printf("     - Artifacts: %v\n", result.Artifacts)
}

func testPOCAgent() {
	agent := agents.NewPOCAgent("test-poc-agent")
	
	workflow := &models.Workflow{
		ID:        "wf-test-poc",
		Type:      models.AgentTypePOC,
		AgentID:   agent.GetID(),
		Variables: map[string]interface{}{
			"language":  "rust",
			"framework": "tokio",
			"feature":   "async server",
		},
	}
	
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	
	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		fmt.Printf("   ❌ POC agent failed: %v\n", err)
		return
	}
	
	fmt.Printf("   ✓ POC agent completed\n")
	fmt.Printf("     - Confidence: %.2f\n", result.ConfidenceScore)
	fmt.Printf("     - Quality: %.2f\n", result.QualityScore)
	fmt.Printf("     - Duration: %dms\n", result.ExecutionTimeMs)
	fmt.Printf("     - Artifacts: %v\n", result.Artifacts)
}

func testDocumentationAgent() {
	agent := agents.NewDocumentationAgent("test-doc-agent")
	
	workflow := &models.Workflow{
		ID:        "wf-test-docs",
		Type:      models.AgentTypeDocumentation,
		AgentID:   agent.GetID(),
		Variables: map[string]interface{}{
			"project_name": "Test Project",
			"source_path":  "./src",
		},
	}
	
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	
	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		fmt.Printf("   ❌ Documentation agent failed: %v\n", err)
		return
	}
	
	fmt.Printf("   ✓ Documentation agent completed\n")
	fmt.Printf("     - Confidence: %.2f\n", result.ConfidenceScore)
	fmt.Printf("     - Quality: %.2f\n", result.QualityScore)
	fmt.Printf("     - Duration: %dms\n", result.ExecutionTimeMs)
	fmt.Printf("     - Artifacts: %v\n", result.Artifacts)
}

func testValidationAgent() {
	agent := agents.NewValidationAgent("test-validation-agent")
	
	workflow := &models.Workflow{
		ID:        "wf-test-validation",
		Type:      models.AgentTypeValidation,
		AgentID:   agent.GetID(),
		Variables: map[string]interface{}{
			"checklist_id": "default",
		},
	}
	
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	
	result, err := agent.Execute(ctx, workflow)
	if err != nil {
		fmt.Printf("   ❌ Validation agent failed: %v\n", err)
		return
	}
	
	fmt.Printf("   ✓ Validation agent completed\n")
	fmt.Printf("     - Confidence: %.2f\n", result.ConfidenceScore)
	fmt.Printf("     - Quality: %.2f\n", result.QualityScore)
	fmt.Printf("     - Duration: %dms\n", result.ExecutionTimeMs)
	fmt.Printf("     - Artifacts: %v\n", result.Artifacts)
}

func testTemplates() {
	tm := agents.NewTemplateManager()
	
	// List all templates
	templates := tm.ListTemplates("")
	fmt.Printf("   ✓ Loaded %d templates\n", len(templates))
	
	// Show templates by type
	types := map[string]int{}
	for _, t := range templates {
		types[t.AgentType]++
	}
	
	for agentType, count := range types {
		fmt.Printf("     - %s: %d templates\n", agentType, count)
	}
	
	// Test applying a template
	template, err := tm.GetTemplate("research-basic")
	if err != nil {
		fmt.Printf("   ❌ Failed to get template: %v\n", err)
		return
	}
	
	fmt.Printf("   ✓ Retrieved template: %s\n", template.Name)
	fmt.Printf("     - Agent Type: %s\n", template.AgentType)
	fmt.Printf("     - Steps: %d\n", len(template.Steps))
}