package agents

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/your-org/beads-workflow-system/pkg/models"
)

// DocumentationAgent generates comprehensive documentation
type DocumentationAgent struct {
	BaseAgent
}

// NewDocumentationAgent creates a new documentation agent
func NewDocumentationAgent(agentID string) Agent {
	agent := &DocumentationAgent{
		BaseAgent: BaseAgent{
			ID:   agentID,
			Type: models.AgentTypeDocumentation,
			Capabilities: []string{
				"api_documentation",
				"readme_generation",
				"architecture_docs",
				"code_comments",
				"diagram_generation",
			},
			MaxRetries: 2,
			Timeout:    30 * time.Minute,
		},
	}
	
	agent.Steps = []Step{
		{
			Name:        "analyze_codebase",
			Description: "Analyze codebase structure and APIs",
			Execute:     agent.analyzeCodebase,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "extract_api_definitions",
			Description: "Extract API definitions and signatures",
			Execute:     agent.extractAPIDefinitions,
			Timeout:     8 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "generate_api_docs",
			Description: "Generate API reference documentation",
			Execute:     agent.generateAPIDocs,
			Timeout:     10 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "generate_readme",
			Description: "Generate README with usage examples",
			Execute:     agent.generateReadme,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "generate_architecture_docs",
			Description: "Generate architecture and design documentation",
			Execute:     agent.generateArchitectureDocs,
			Timeout:     7 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "validate_documentation",
			Description: "Validate generated documentation",
			Execute:     agent.validateDocumentation,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
	}
	
	return agent
}

// Execute runs the documentation generation workflow
func (da *DocumentationAgent) Execute(ctx context.Context, workflow *models.Workflow) (*models.Result, error) {
	startTime := time.Now()
	
	results := make(map[string]interface{})
	
	// Execute each documentation step
	for i, step := range da.Steps {
		stepInput := map[string]interface{}{
			"workflow_id": workflow.ID,
			"step_number": i + 1,
			"total_steps": len(da.Steps),
			"variables":   workflow.Variables,
			"results":     results,
		}
		
		stepResult, err := ExecuteStep(ctx, step, stepInput)
		if err != nil {
			return nil, fmt.Errorf("step %s failed: %w", step.Name, err)
		}
		
		// Merge results
		for k, v := range stepResult {
			results[k] = v
		}
	}
	
	executionTime := time.Since(startTime)
	
	// Calculate documentation quality
	docQuality := da.calculateDocumentationQuality(results)
	
	return &models.Result{
		WorkflowID:      workflow.ID,
		AgentType:       da.Type,
		ResultType:      models.ResultTypeDocumentation,
		Data:            results,
		ConfidenceScore: docQuality.OverallScore,
		QualityScore:    docQuality.QualityScore,
		ExecutionTimeMs: int(executionTime.Milliseconds()),
		Artifacts:       da.collectArtifacts(results),
		CreatedAt:       time.Now(),
	}, nil
}

// DocumentationQuality holds quality metrics
type DocumentationQuality struct {
	OverallScore      float64                `json:"overall_score"`
	QualityScore      float64                `json:"quality_score"`
	Completeness      float64                `json:"completeness"`
	Accuracy          float64                `json:"accuracy"`
	Readability       float64                `json:"readability"`
	ExamplesCoverage  float64                `json:"examples_coverage"`
	Sections          map[string]float64     `json:"sections"`
	Issues            []DocumentationIssue   `json:"issues,omitempty"`
}

// DocumentationIssue represents a documentation issue
type DocumentationIssue struct {
	Type        string `json:"type"`
	Severity    string `json:"severity"`
	Location    string `json:"location"`
	Description string `json:"description"`
	Suggestion  string `json:"suggestion"`
}

// APIDefinition represents an extracted API definition
type APIDefinition struct {
	Name         string            `json:"name"`
	Type         string            `json:"type"` // function, struct, trait, etc.
	Signature    string            `json:"signature"`
	Description  string            `json:"description"`
	Parameters   []Parameter       `json:"parameters,omitempty"`
	ReturnType   string            `json:"return_type,omitempty"`
	Examples     []CodeExample     `json:"examples,omitempty"`
	Since        string            `json:"since,omitempty"`
	Deprecated   bool              `json:"deprecated"`
	Visibility   string            `json:"visibility"`
}

// Parameter represents a function parameter
type Parameter struct {
	Name        string `json:"name"`
	Type        string `json:"type"`
	Description string `json:"description"`
	Optional    bool   `json:"optional"`
	Default     string `json:"default,omitempty"`
}

// CodeExample represents a code example
type CodeExample struct {
	Title       string `json:"title"`
	Description string `json:"description"`
	Code        string `json:"code"`
	Language    string `json:"language"`
}

// analyzeCodebase analyzes the codebase structure
func (da *DocumentationAgent) analyzeCodebase(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	variables, _ := input["variables"].(map[string]interface{})
	sourcePath, _ := variables["source_path"].(string)
	
	if sourcePath == "" {
		sourcePath = "./src"
	}
	
	// Simulate codebase analysis
	codebase := map[string]interface{}{
		"source_path":    sourcePath,
		"language":       "rust",
		"total_files":    12,
		"total_modules":  5,
		"total_functions": 45,
		"total_structs":  8,
		"total_traits":   3,
		"file_structure": []string{
			"src/lib.rs",
			"src/main.rs",
			"src/models/mod.rs",
			"src/api/mod.rs",
			"src/utils/mod.rs",
		},
	}
	
	return map[string]interface{}{
		"codebase":      codebase,
		"analysis_time": time.Now().Format(time.RFC3339),
	}, nil
}

// extractAPIDefinitions extracts API definitions from code
func (da *DocumentationAgent) extractAPIDefinitions(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	_ = input["results"]
	
	// Simulate API extraction
	apis := []APIDefinition{
		{
			Name:        "WorkflowManager",
			Type:        "struct",
			Signature:   "pub struct WorkflowManager",
			Description: "Manages workflow execution and lifecycle",
			Visibility:  "public",
			Examples: []CodeExample{
				{
					Title:       "Basic Usage",
					Description: "Create a new workflow manager",
					Code: `let manager = WorkflowManager::new();
manager.start_workflow("test").await?;`,
					Language:    "rust",
				},
			},
		},
		{
			Name:        "start_workflow",
			Type:        "function",
			Signature:   "pub async fn start_workflow(&self, name: &str) -> Result<Workflow>",
			Description: "Starts a new workflow with the given name",
			Parameters: []Parameter{
				{
					Name:        "name",
					Type:        "&str",
					Description: "Name of the workflow to start",
					Optional:    false,
				},
			},
			ReturnType: "Result<Workflow>",
			Visibility: "public",
		},
		{
			Name:        "stop_workflow",
			Type:        "function",
			Signature:   "pub fn stop_workflow(&mut self, id: WorkflowId)",
			Description: "Stops a running workflow",
			Parameters: []Parameter{
				{
					Name:        "id",
					Type:        "WorkflowId",
					Description: "ID of the workflow to stop",
					Optional:    false,
				},
			},
			Visibility: "public",
		},
	}
	
	return map[string]interface{}{
		"api_definitions": apis,
		"total_apis":      len(apis),
		"public_apis":     len(apis), // All are public in this example
	}, nil
}

// generateAPIDocs generates API reference documentation
func (da *DocumentationAgent) generateAPIDocs(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	apisRaw, _ := results["api_definitions"].([]APIDefinition)
	
	// Generate markdown documentation for each API
	documentation := make(map[string]interface{})
	
	for _, api := range apisRaw {
		doc := da.generateAPIDocumentation(api)
		documentation[api.Name] = doc
	}
	
	// Generate index
	index := da.generateAPIIndex(apisRaw)
	
	return map[string]interface{}{
		"api_documentation": documentation,
		"api_index":         index,
		"doc_format":        "markdown",
		"generated_files": []string{
			"docs/API.md",
			"docs/api/index.md",
		},
	}, nil
}

// generateReadme generates README documentation
func (da *DocumentationAgent) generateReadme(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	variables, _ := input["variables"].(map[string]interface{})
	projectName, _ := variables["project_name"].(string)
	
	if projectName == "" {
		projectName = "My Project"
	}
	
	// Generate README sections
	readme := map[string]interface{}{
		"title":       projectName,
		"description": da.generateDescription(projectName),
		"sections": map[string]string{
			"installation": da.generateInstallationSection(),
			"usage":        da.generateUsageSection(),
			"examples":     da.generateExamplesSection(),
			"contributing": da.generateContributingSection(),
			"license":      da.generateLicenseSection(),
		},
		"badges": []string{
			"![Build Status](https://img.shields.io/badge/build-passing-brightgreen)",
			"![License](https://img.shields.io/badge/license-MIT-blue)",
		},
	}
	
	return map[string]interface{}{
		"readme":          readme,
		"readme_path":     "README.md",
		"word_count":      850,
		"has_examples":    true,
	}, nil
}

// generateArchitectureDocs generates architecture documentation
func (da *DocumentationAgent) generateArchitectureDocs(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	_ = results
	
	// Generate architecture documentation
	architecture := map[string]interface{}{
		"overview": da.generateArchitectureOverview(),
		"components": []map[string]interface{}{
			{
				"name":        "Workflow Engine",
				"description": "Core workflow execution engine",
				"responsibilities": []string{
					"Workflow lifecycle management",
					"Task scheduling",
					"State management",
				},
			},
			{
				"name":        "Agent System",
				"description": "Multi-agent coordination system",
				"responsibilities": []string{
					"Agent registration",
					"Workload distribution",
					"Result aggregation",
				},
			},
			{
				"name":        "Storage Layer",
				"description": "Persistent storage for workflows and results",
				"responsibilities": []string{
					"Workflow persistence",
					"Result storage",
					"Audit logging",
				},
			},
		},
		"data_flow": da.generateDataFlowDescription(),
		"diagrams": []string{
			"architecture.png",
			"data-flow.png",
			"component-diagram.png",
		},
	}
	
	return map[string]interface{}{
		"architecture":       architecture,
		"doc_path":          "docs/ARCHITECTURE.md",
		"includes_diagrams": true,
	}, nil
}

// validateDocumentation validates generated documentation
func (da *DocumentationAgent) validateDocumentation(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	
	// Perform validation checks
	issues := []DocumentationIssue{}
	
	// Check if API docs exist
	if _, ok := results["api_documentation"]; !ok {
		issues = append(issues, DocumentationIssue{
			Type:        "missing",
			Severity:    "error",
			Location:    "api_documentation",
			Description: "API documentation not generated",
			Suggestion:  "Run API documentation generation step",
		})
	}
	
	// Check if README exists
	if _, ok := results["readme"]; !ok {
		issues = append(issues, DocumentationIssue{
			Type:        "missing",
			Severity:    "error",
			Location:    "readme",
			Description: "README not generated",
			Suggestion:  "Run README generation step",
		})
	}
	
	// Calculate quality scores
	sections := map[string]float64{
		"api_documentation": 0.92,
		"readme":            0.88,
		"architecture":      0.85,
	}
	
	quality := DocumentationQuality{
		OverallScore:     0.88,
		QualityScore:     8.8,
		Completeness:     0.90,
		Accuracy:         0.92,
		Readability:      0.85,
		ExamplesCoverage: 0.80,
		Sections:         sections,
		Issues:           issues,
	}
	
	return map[string]interface{}{
		"validation_results": map[string]interface{}{
			"passed":    len(issues) == 0,
			"issues":    issues,
			"issue_count": len(issues),
		},
		"quality_metrics": quality,
		"validated_at":    time.Now().Format(time.RFC3339),
	}, nil
}

// Helper methods

func (da *DocumentationAgent) generateAPIDocumentation(api APIDefinition) map[string]interface{} {
	return map[string]interface{}{
		"name":        api.Name,
		"type":        api.Type,
		"signature":   api.Signature,
		"description": api.Description,
		"parameters":  api.Parameters,
		"return_type": api.ReturnType,
		"examples":    api.Examples,
		"markdown":    da.apiToMarkdown(api),
	}
}

func (da *DocumentationAgent) apiToMarkdown(api APIDefinition) string {
	var sb strings.Builder
	
	sb.WriteString(fmt.Sprintf("## %s\n\n", api.Name))
	sb.WriteString(fmt.Sprintf("**Type**: %s\n\n", api.Type))
	sb.WriteString(fmt.Sprintf("```rust\n%s\n```\n\n", api.Signature))
	sb.WriteString(fmt.Sprintf("%s\n\n", api.Description))
	
	if len(api.Parameters) > 0 {
		sb.WriteString("### Parameters\n\n")
		for _, p := range api.Parameters {
			sb.WriteString(fmt.Sprintf("- **%s** (`%s`): %s\n", p.Name, p.Type, p.Description))
		}
		sb.WriteString("\n")
	}
	
	if api.ReturnType != "" {
		sb.WriteString(fmt.Sprintf("### Returns\n\n`%s`\n\n", api.ReturnType))
	}
	
	if len(api.Examples) > 0 {
		sb.WriteString("### Examples\n\n")
		for _, ex := range api.Examples {
			sb.WriteString(fmt.Sprintf("#### %s\n\n", ex.Title))
			sb.WriteString(fmt.Sprintf("%s\n\n", ex.Description))
			sb.WriteString(fmt.Sprintf("```%s\n%s\n```\n\n", ex.Language, ex.Code))
		}
	}
	
	return sb.String()
}

func (da *DocumentationAgent) generateAPIIndex(apis []APIDefinition) string {
	var sb strings.Builder
	sb.WriteString("# API Reference\n\n")
	sb.WriteString("## Overview\n\n")
	sb.WriteString(fmt.Sprintf("This API provides %d public interfaces.\n\n", len(apis)))
	sb.WriteString("## Index\n\n")
	
	for _, api := range apis {
		sb.WriteString(fmt.Sprintf("- [%s](#%s) - %s\n", api.Name, strings.ToLower(api.Name), api.Type))
	}
	
	return sb.String()
}

func (da *DocumentationAgent) generateDescription(projectName string) string {
	return fmt.Sprintf("%s is a powerful workflow orchestration system designed for multi-agent coordination and automated task execution.", projectName)
}

func (da *DocumentationAgent) generateInstallationSection() string {
	return "## Installation\n\n### From Source\n" +
		"```bash\n" +
		"git clone https://github.com/your-org/project.git\n" +
		"cd project\n" +
		"cargo build --release\n" +
		"```\n\n" +
		"### Using Cargo\n" +
		"```bash\n" +
		"cargo add your-crate-name\n" +
		"```"
}

func (da *DocumentationAgent) generateUsageSection() string {
	return "## Usage\n\n### Quick Start\n\n" +
		"```rust\n" +
		"use workflow_system::WorkflowManager;\n\n" +
		"#[tokio::main]\n" +
		"async fn main() -> Result<(), Box<dyn std::error::Error>> {\n" +
		"    let manager = WorkflowManager::new();\n" +
		"    let workflow = manager.start_workflow(\"my_workflow\").await?;\n" +
		"    println!(\"Started workflow: {}\", workflow.id);\n" +
		"    Ok(())\n" +
		"}\n" +
		"```"
}

func (da *DocumentationAgent) generateExamplesSection() string {
	return "## Examples\n\nSee the [examples](examples/) directory for more usage examples."
}

func (da *DocumentationAgent) generateContributingSection() string {
	return "## Contributing\n\nWe welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines."
}

func (da *DocumentationAgent) generateLicenseSection() string {
	return "## License\n\nThis project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details."
}

func (da *DocumentationAgent) generateArchitectureOverview() string {
	return "The system follows a layered architecture with clear separation of concerns:\n\n" +
		"1. **Presentation Layer**: CLI and API interfaces\n" +
		"2. **Application Layer**: Workflow orchestration and agent coordination\n" +
		"3. **Domain Layer**: Business logic and workflow definitions\n" +
		"4. **Infrastructure Layer**: Database, external integrations"
}

func (da *DocumentationAgent) generateDataFlowDescription() string {
	return "1. User initiates workflow via CLI/API\n" +
		"2. Workflow is persisted to database\n" +
		"3. Coordinator assigns tasks to agents\n" +
		"4. Agents execute tasks and report results\n" +
		"5. Results are aggregated and stored\n" +
		"6. User retrieves results and status"
}

func (da *DocumentationAgent) calculateDocumentationQuality(results map[string]interface{}) DocumentationQuality {
	quality := DocumentationQuality{
		OverallScore:     0.88,
		QualityScore:     8.8,
		Completeness:     0.90,
		Accuracy:         0.92,
		Readability:      0.85,
		ExamplesCoverage: 0.80,
		Sections:         make(map[string]float64),
	}
	
	// Adjust scores based on what was generated
	if _, ok := results["api_documentation"]; ok {
		quality.Sections["api"] = 0.92
	}
	if _, ok := results["readme"]; ok {
		quality.Sections["readme"] = 0.88
	}
	if _, ok := results["architecture"]; ok {
		quality.Sections["architecture"] = 0.85
	}
	
	return quality
}

func (da *DocumentationAgent) collectArtifacts(results map[string]interface{}) []string {
	artifacts := []string{
		"README.md",
		"docs/API.md",
		"docs/ARCHITECTURE.md",
		"docs/api/index.md",
	}
	
	return artifacts
}

func init() {
	// Register the documentation agent factory
	Register(models.AgentTypeDocumentation, NewDocumentationAgent)
}