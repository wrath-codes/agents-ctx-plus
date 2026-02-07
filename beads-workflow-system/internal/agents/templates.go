package agents

import (
	"encoding/json"
	"fmt"

	"github.com/your-org/beads-workflow-system/pkg/models"
)

// WorkflowTemplate defines a reusable workflow template
type WorkflowTemplate struct {
	ID          string                 `json:"id"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	AgentType   string                 `json:"agent_type"`
	Steps       []TemplateStep         `json:"steps"`
	Variables   map[string]interface{} `json:"variables"`
	Config      TemplateConfig         `json:"config"`
}

// TemplateStep defines a step in a workflow template
type TemplateStep struct {
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Timeout     string                 `json:"timeout"`
	RetryCount  int                    `json:"retry_count"`
	Parameters  map[string]interface{} `json:"parameters"`
}

// TemplateConfig holds template configuration
type TemplateConfig struct {
	MaxRetries    int    `json:"max_retries"`
	Timeout       string `json:"timeout"`
	ContinueOnError bool `json:"continue_on_error"`
}

// TemplateManager manages workflow templates
type TemplateManager struct {
	templates map[string]WorkflowTemplate
}

// NewTemplateManager creates a new template manager
func NewTemplateManager() *TemplateManager {
	tm := &TemplateManager{
		templates: make(map[string]WorkflowTemplate),
	}
	
	// Register default templates
	tm.registerDefaultTemplates()
	
	return tm
}

// GetTemplate retrieves a template by ID
func (tm *TemplateManager) GetTemplate(id string) (WorkflowTemplate, error) {
	template, exists := tm.templates[id]
	if !exists {
		return WorkflowTemplate{}, fmt.Errorf("template not found: %s", id)
	}
	return template, nil
}

// ListTemplates lists all available templates
func (tm *TemplateManager) ListTemplates(agentType string) []WorkflowTemplate {
	var templates []WorkflowTemplate
	
	for _, template := range tm.templates {
		if agentType == "" || template.AgentType == agentType {
			templates = append(templates, template)
		}
	}
	
	return templates
}

// RegisterTemplate registers a new template
func (tm *TemplateManager) RegisterTemplate(template WorkflowTemplate) error {
	if template.ID == "" {
		return fmt.Errorf("template ID is required")
	}
	
	tm.templates[template.ID] = template
	return nil
}

// ApplyTemplate applies a template to create a workflow request
func (tm *TemplateManager) ApplyTemplate(templateID string, variables map[string]interface{}) (*models.StartWorkflowRequest, error) {
	template, err := tm.GetTemplate(templateID)
	if err != nil {
		return nil, err
	}
	
	// Merge template variables with provided variables
	mergedVars := make(map[string]interface{})
	for k, v := range template.Variables {
		mergedVars[k] = v
	}
	for k, v := range variables {
		mergedVars[k] = v
	}
	
	return &models.StartWorkflowRequest{
		IssueTitle:   template.Name,
		WorkflowType: template.AgentType,
		AgentType:    template.AgentType,
		Priority:     2, // Default priority
		Variables:    mergedVars,
		TemplateID:   templateID,
	}, nil
}

// registerDefaultTemplates registers the built-in templates
func (tm *TemplateManager) registerDefaultTemplates() {
	// Research templates
	tm.templates["research-basic"] = WorkflowTemplate{
		ID:          "research-basic",
		Name:        "Basic Research",
		Description: "Basic library research with discovery and analysis",
		AgentType:   models.AgentTypeResearch,
		Steps: []TemplateStep{
			{
				Name:        "library_discovery",
				Description: "Discover relevant libraries",
				Timeout:     "5m",
				RetryCount:  2,
			},
			{
				Name:        "documentation_analysis",
				Description: "Analyze documentation",
				Timeout:     "10m",
				RetryCount:  2,
			},
			{
				Name:        "findings_synthesis",
				Description: "Synthesize findings",
				Timeout:     "5m",
				RetryCount:  1,
			},
		},
		Variables: map[string]interface{}{
			"focus": "general",
		},
		Config: TemplateConfig{
			MaxRetries:      3,
			Timeout:         "30m",
			ContinueOnError: false,
		},
	}
	
	tm.templates["research-performance"] = WorkflowTemplate{
		ID:          "research-performance",
		Name:        "Performance Research",
		Description: "Research focused on performance benchmarks and comparisons",
		AgentType:   models.AgentTypeResearch,
		Steps: []TemplateStep{
			{
				Name:        "library_discovery",
				Description: "Discover libraries with focus on performance",
				Timeout:     "5m",
				RetryCount:  2,
				Parameters: map[string]interface{}{
					"criteria": []string{"performance", "benchmarks"},
				},
			},
			{
				Name:        "documentation_analysis",
				Description: "Analyze performance characteristics",
				Timeout:     "10m",
				RetryCount:  2,
			},
			{
				Name:        "static_analysis",
				Description: "Analyze implementation efficiency",
				Timeout:     "8m",
				RetryCount:  1,
			},
			{
				Name:        "findings_synthesis",
				Description: "Generate performance comparison report",
				Timeout:     "5m",
				RetryCount:  1,
			},
		},
		Variables: map[string]interface{}{
			"focus": "performance",
		},
		Config: TemplateConfig{
			MaxRetries:      3,
			Timeout:         "30m",
			ContinueOnError: false,
		},
	}
	
	// POC templates
	tm.templates["poc-basic"] = WorkflowTemplate{
		ID:          "poc-basic",
		Name:        "Basic POC",
		Description: "Basic proof-of-concept with build and test",
		AgentType:   models.AgentTypePOC,
		Steps: []TemplateStep{
			{
				Name:        "setup_environment",
				Description: "Setup development environment",
				Timeout:     "5m",
				RetryCount:  2,
			},
			{
				Name:        "generate_implementation",
				Description: "Generate POC code",
				Timeout:     "10m",
				RetryCount:  2,
			},
			{
				Name:        "build_code",
				Description: "Build the code",
				Timeout:     "10m",
				RetryCount:  1,
			},
			{
				Name:        "run_tests",
				Description: "Run test suite",
				Timeout:     "10m",
				RetryCount:  1,
			},
			{
				Name:        "cleanup_and_report",
				Description: "Generate report",
				Timeout:     "5m",
				RetryCount:  1,
			},
		},
		Variables: map[string]interface{}{
			"language": "rust",
		},
		Config: TemplateConfig{
			MaxRetries:      3,
			Timeout:         "45m",
			ContinueOnError: false,
		},
	}
	
	tm.templates["poc-full"] = WorkflowTemplate{
		ID:          "poc-full",
		Name:        "Full POC with Benchmarks",
		Description: "Complete POC including performance benchmarks",
		AgentType:   models.AgentTypePOC,
		Steps: []TemplateStep{
			{
				Name:        "setup_environment",
				Description: "Setup development environment",
				Timeout:     "5m",
				RetryCount:  2,
			},
			{
				Name:        "generate_implementation",
				Description: "Generate POC code",
				Timeout:     "10m",
				RetryCount:  2,
			},
			{
				Name:        "build_code",
				Description: "Build the code",
				Timeout:     "10m",
				RetryCount:  1,
			},
			{
				Name:        "run_tests",
				Description: "Run test suite",
				Timeout:     "10m",
				RetryCount:  1,
			},
			{
				Name:        "benchmark_performance",
				Description: "Run performance benchmarks",
				Timeout:     "8m",
				RetryCount:  1,
			},
			{
				Name:        "cleanup_and_report",
				Description: "Generate comprehensive report",
				Timeout:     "5m",
				RetryCount:  1,
			},
		},
		Variables: map[string]interface{}{
			"language":   "rust",
			"benchmarks": true,
		},
		Config: TemplateConfig{
			MaxRetries:      3,
			Timeout:         "50m",
			ContinueOnError: false,
		},
	}
	
	// Documentation templates
	tm.templates["docs-basic"] = WorkflowTemplate{
		ID:          "docs-basic",
		Name:        "Basic Documentation",
		Description: "Generate README and basic API docs",
		AgentType:   models.AgentTypeDocumentation,
		Steps: []TemplateStep{
			{
				Name:        "analyze_codebase",
				Description: "Analyze codebase structure",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "extract_api_definitions",
				Description: "Extract API definitions",
				Timeout:     "8m",
				RetryCount:  1,
			},
			{
				Name:        "generate_readme",
				Description: "Generate README",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "validate_documentation",
				Description: "Validate documentation",
				Timeout:     "5m",
				RetryCount:  1,
			},
		},
		Variables: map[string]interface{}{
			"include_examples": true,
		},
		Config: TemplateConfig{
			MaxRetries:      2,
			Timeout:         "30m",
			ContinueOnError: false,
		},
	}
	
	tm.templates["docs-comprehensive"] = WorkflowTemplate{
		ID:          "docs-comprehensive",
		Name:        "Comprehensive Documentation",
		Description: "Generate complete documentation including architecture docs",
		AgentType:   models.AgentTypeDocumentation,
		Steps: []TemplateStep{
			{
				Name:        "analyze_codebase",
				Description: "Analyze codebase structure",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "extract_api_definitions",
				Description: "Extract API definitions",
				Timeout:     "8m",
				RetryCount:  1,
			},
			{
				Name:        "generate_api_docs",
				Description: "Generate API reference",
				Timeout:     "10m",
				RetryCount:  1,
			},
			{
				Name:        "generate_readme",
				Description: "Generate README",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "generate_architecture_docs",
				Description: "Generate architecture documentation",
				Timeout:     "7m",
				RetryCount:  1,
			},
			{
				Name:        "validate_documentation",
				Description: "Validate all documentation",
				Timeout:     "5m",
				RetryCount:  1,
			},
		},
		Variables: map[string]interface{}{
			"include_examples":     true,
			"include_architecture": true,
			"include_diagrams":     true,
		},
		Config: TemplateConfig{
			MaxRetries:      2,
			Timeout:         "35m",
			ContinueOnError: false,
		},
	}
	
	// Validation templates
	tm.templates["validation-standard"] = WorkflowTemplate{
		ID:          "validation-standard",
		Name:        "Standard Validation",
		Description: "Standard validation with code quality, security, and performance checks",
		AgentType:   models.AgentTypeValidation,
		Steps: []TemplateStep{
			{
				Name:        "load_checklist",
				Description: "Load validation checklist",
				Timeout:     "2m",
				RetryCount:  1,
			},
			{
				Name:        "validate_code_quality",
				Description: "Validate code quality",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "validate_security",
				Description: "Security validation",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "generate_validation_report",
				Description: "Generate report",
				Timeout:     "5m",
				RetryCount:  1,
			},
		},
		Variables: map[string]interface{}{
			"checklist_id": "default",
		},
		Config: TemplateConfig{
			MaxRetries:      1,
			Timeout:         "20m",
			ContinueOnError: true,
		},
	}
	
	tm.templates["validation-full"] = WorkflowTemplate{
		ID:          "validation-full",
		Name:        "Full Validation",
		Description: "Comprehensive validation including all categories",
		AgentType:   models.AgentTypeValidation,
		Steps: []TemplateStep{
			{
				Name:        "load_checklist",
				Description: "Load validation checklist",
				Timeout:     "2m",
				RetryCount:  1,
			},
			{
				Name:        "validate_code_quality",
				Description: "Validate code quality",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "validate_security",
				Description: "Security validation",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "validate_performance",
				Description: "Performance validation",
				Timeout:     "5m",
				RetryCount:  1,
			},
			{
				Name:        "validate_compliance",
				Description: "Compliance checks",
				Timeout:     "3m",
				RetryCount:  1,
			},
			{
				Name:        "generate_validation_report",
				Description: "Generate comprehensive report",
				Timeout:     "5m",
				RetryCount:  1,
			},
		},
		Variables: map[string]interface{}{
			"checklist_id": "default",
		},
		Config: TemplateConfig{
			MaxRetries:      1,
			Timeout:         "25m",
			ContinueOnError: true,
		},
	}
}

// ToJSON converts template to JSON
func (t WorkflowTemplate) ToJSON() (string, error) {
	bytes, err := json.MarshalIndent(t, "", "  ")
	if err != nil {
		return "", err
	}
	return string(bytes), nil
}

// FromJSON creates template from JSON
func (t *WorkflowTemplate) FromJSON(data string) error {
	return json.Unmarshal([]byte(data), t)
}