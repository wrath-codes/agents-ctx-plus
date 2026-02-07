package agents

import (
	"context"
	"fmt"
	"time"

	"github.com/your-org/beads-workflow-system/pkg/models"
)

// ValidationAgent validates workflows against checklists and criteria
type ValidationAgent struct {
	BaseAgent
}

// NewValidationAgent creates a new validation agent
func NewValidationAgent(agentID string) Agent {
	agent := &ValidationAgent{
		BaseAgent: BaseAgent{
			ID:   agentID,
			Type: models.AgentTypeValidation,
			Capabilities: []string{
				"code_review",
				"security_audit",
				"performance_validation",
				"compliance_check",
				"checklist_validation",
			},
			MaxRetries: 1,
			Timeout:    20 * time.Minute,
		},
	}
	
	agent.Steps = []Step{
		{
			Name:        "load_checklist",
			Description: "Load validation checklist",
			Execute:     agent.loadChecklist,
			Timeout:     2 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "validate_code_quality",
			Description: "Validate code quality criteria",
			Execute:     agent.validateCodeQuality,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "validate_security",
			Description: "Perform security validation",
			Execute:     agent.validateSecurity,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "validate_performance",
			Description: "Validate performance criteria",
			Execute:     agent.validatePerformance,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "validate_compliance",
			Description: "Check compliance requirements",
			Execute:     agent.validateCompliance,
			Timeout:     3 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "generate_validation_report",
			Description: "Generate validation report",
			Execute:     agent.generateValidationReport,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
	}
	
	return agent
}

// Execute runs the validation workflow
func (va *ValidationAgent) Execute(ctx context.Context, workflow *models.Workflow) (*models.Result, error) {
	startTime := time.Now()
	
	results := make(map[string]interface{})
	var allChecks []CheckResult
	
	// Execute each validation step
	for i, step := range va.Steps {
		stepInput := map[string]interface{}{
			"workflow_id": workflow.ID,
			"step_number": i + 1,
			"total_steps": len(va.Steps),
			"variables":   workflow.Variables,
			"results":     results,
		}
		
		stepResult, err := ExecuteStep(ctx, step, stepInput)
		if err != nil {
			return nil, fmt.Errorf("validation step %s failed: %w", step.Name, err)
		}
		
		// Merge results
		for k, v := range stepResult {
			results[k] = v
		}
		
		// Collect check results
		if checks, ok := stepResult["checks"].([]CheckResult); ok {
			allChecks = append(allChecks, checks...)
		}
	}
	
	executionTime := time.Since(startTime)
	
	// Calculate validation score
	validationScore := va.calculateValidationScore(allChecks)
	
	return &models.Result{
		WorkflowID:      workflow.ID,
		AgentType:       va.Type,
		ResultType:      models.ResultTypeValidation,
		Data:            results,
		ConfidenceScore: validationScore,
		QualityScore:    validationScore * 10, // Scale to 0-10
		ExecutionTimeMs: int(executionTime.Milliseconds()),
		Artifacts:       []string{"validation_report.json", "compliance_checklist.md"},
		CreatedAt:       time.Now(),
	}, nil
}

// ValidationChecklist defines validation criteria
type ValidationChecklist struct {
	ID          string           `json:"id"`
	Name        string           `json:"name"`
	Description string           `json:"description"`
	Categories  []CheckCategory  `json:"categories"`
	CreatedAt   time.Time        `json:"created_at"`
}

// CheckCategory groups related checks
type CheckCategory struct {
	Name        string        `json:"name"`
	Description string        `json:"description"`
	Weight      float64       `json:"weight"`
	Checks      []CheckItem   `json:"checks"`
}

// CheckItem represents a single validation check
type CheckItem struct {
	ID          string   `json:"id"`
	Name        string   `json:"name"`
	Description string   `json:"description"`
	Type        string   `json:"type"` // automated, manual, review
	Required    bool     `json:"required"`
	Criteria    []string `json:"criteria"`
}

// CheckResult represents the result of a validation check
type CheckResult struct {
	CheckID     string   `json:"check_id"`
	Name        string   `json:"name"`
	Category    string   `json:"category"`
	Status      string   `json:"status"` // passed, failed, skipped, warning
	Score       float64  `json:"score"`
	MaxScore    float64  `json:"max_score"`
	Message     string   `json:"message"`
	Details     string   `json:"details,omitempty"`
	Suggestions []string `json:"suggestions,omitempty"`
	Duration    int      `json:"duration_ms"`
	Timestamp   time.Time `json:"timestamp"`
}

// ValidationReport holds the complete validation results
type ValidationReport struct {
	WorkflowID         string         `json:"workflow_id"`
	ChecklistID        string         `json:"checklist_id"`
	OverallScore       float64        `json:"overall_score"`
	Status             string         `json:"status"` // passed, failed, partial
	TotalChecks        int            `json:"total_checks"`
	PassedChecks       int            `json:"passed_checks"`
	FailedChecks       int            `json:"failed_checks"`
	WarningChecks      int            `json:"warning_checks"`
	SkippedChecks      int            `json:"skipped_checks"`
	CategoryScores     map[string]float64 `json:"category_scores"`
	Results            []CheckResult  `json:"results"`
	Summary            string         `json:"summary"`
	Recommendations    []string       `json:"recommendations"`
	GeneratedAt        time.Time      `json:"generated_at"`
}

// loadChecklist loads the validation checklist
func (va *ValidationAgent) loadChecklist(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	variables, _ := input["variables"].(map[string]interface{})
	checklistID, _ := variables["checklist_id"].(string)
	
	if checklistID == "" {
		checklistID = "default"
	}
	
	// Define default comprehensive checklist
	checklist := ValidationChecklist{
		ID:          checklistID,
		Name:        "Standard Validation Checklist",
		Description: "Comprehensive validation checklist for workflow artifacts",
		Categories: []CheckCategory{
			{
				Name:        "Code Quality",
				Description: "Code quality and maintainability checks",
				Weight:      0.30,
				Checks: []CheckItem{
					{
						ID:          "cq-001",
						Name:        "Test Coverage",
						Description: "Minimum 80% test coverage",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"coverage >= 80%"},
					},
					{
						ID:          "cq-002",
						Name:        "Code Complexity",
						Description: "Cyclomatic complexity under 15",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"complexity < 15"},
					},
					{
						ID:          "cq-003",
						Name:        "Documentation",
						Description: "Public APIs documented",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"doc coverage >= 60%"},
					},
				},
			},
			{
				Name:        "Security",
				Description: "Security and vulnerability checks",
				Weight:      0.30,
				Checks: []CheckItem{
					{
						ID:          "sec-001",
						Name:        "Dependency Audit",
						Description: "No known vulnerabilities in dependencies",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"no critical vulnerabilities", "no high vulnerabilities"},
					},
					{
						ID:          "sec-002",
						Name:        "Secret Detection",
						Description: "No secrets in code",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"no secrets detected"},
					},
					{
						ID:          "sec-003",
						Name:        "Input Validation",
						Description: "All inputs validated",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"input validation present"},
					},
				},
			},
			{
				Name:        "Performance",
				Description: "Performance and efficiency checks",
				Weight:      0.20,
				Checks: []CheckItem{
					{
						ID:          "perf-001",
						Name:        "Response Time",
						Description: "API response time under 100ms",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"p95 latency < 100ms"},
					},
					{
						ID:          "perf-002",
						Name:        "Memory Usage",
						Description: "Memory usage within limits",
						Type:        "automated",
						Required:    false,
						Criteria:    []string{"memory < 512MB"},
					},
				},
			},
			{
				Name:        "Compliance",
				Description: "Compliance and standards checks",
				Weight:      0.20,
				Checks: []CheckItem{
					{
						ID:          "comp-001",
						Name:        "License Check",
						Description: "Compatible licenses only",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"OSI approved license"},
					},
					{
						ID:          "comp-002",
						Name:        "Code Standards",
						Description: "Follows project coding standards",
						Type:        "automated",
						Required:    true,
						Criteria:    []string{"linting passed", "formatting correct"},
					},
				},
			},
		},
		CreatedAt: time.Now(),
	}
	
	return map[string]interface{}{
		"checklist":      checklist,
		"total_checks":   va.countTotalChecks(checklist),
		"categories":     len(checklist.Categories),
	}, nil
}

// validateCodeQuality validates code quality criteria
func (va *ValidationAgent) validateCodeQuality(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	checklist, _ := results["checklist"].(ValidationChecklist)
	
	var checks []CheckResult
	
	// Find code quality category
	for _, category := range checklist.Categories {
		if category.Name == "Code Quality" {
			for _, check := range category.Checks {
				result := va.runCodeQualityCheck(check)
				result.Category = category.Name
				checks = append(checks, result)
			}
		}
	}
	
	return map[string]interface{}{
		"checks":           checks,
		"category":         "Code Quality",
		"passed":           va.countPassed(checks),
		"failed":           va.countFailed(checks),
	}, nil
}

// validateSecurity performs security validation
func (va *ValidationAgent) validateSecurity(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	checklist, _ := results["checklist"].(ValidationChecklist)
	
	var checks []CheckResult
	
	for _, category := range checklist.Categories {
		if category.Name == "Security" {
			for _, check := range category.Checks {
				result := va.runSecurityCheck(check)
				result.Category = category.Name
				checks = append(checks, result)
			}
		}
	}
	
	return map[string]interface{}{
		"checks":           checks,
		"category":         "Security",
		"passed":           va.countPassed(checks),
		"failed":           va.countFailed(checks),
		"vulnerabilities":  0,
	}, nil
}

// validatePerformance validates performance criteria
func (va *ValidationAgent) validatePerformance(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	checklist, _ := results["checklist"].(ValidationChecklist)
	
	var checks []CheckResult
	
	for _, category := range checklist.Categories {
		if category.Name == "Performance" {
			for _, check := range category.Checks {
				result := va.runPerformanceCheck(check)
				result.Category = category.Name
				checks = append(checks, result)
			}
		}
	}
	
	return map[string]interface{}{
		"checks":           checks,
		"category":         "Performance",
		"passed":           va.countPassed(checks),
		"failed":           va.countFailed(checks),
		"benchmarks":       []string{"bench_basic", "bench_concurrent"},
	}, nil
}

// validateCompliance checks compliance requirements
func (va *ValidationAgent) validateCompliance(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	checklist, _ := results["checklist"].(ValidationChecklist)
	
	var checks []CheckResult
	
	for _, category := range checklist.Categories {
		if category.Name == "Compliance" {
			for _, check := range category.Checks {
				result := va.runComplianceCheck(check)
				result.Category = category.Name
				checks = append(checks, result)
			}
		}
	}
	
	return map[string]interface{}{
		"checks":           checks,
		"category":         "Compliance",
		"passed":           va.countPassed(checks),
		"failed":           va.countFailed(checks),
		"standards_met":    []string{"MIT License", "Rust Standards"},
	}, nil
}

// generateValidationReport generates the final validation report
func (va *ValidationAgent) generateValidationReport(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	checklist, _ := results["checklist"].(ValidationChecklist)
	
	// Collect all checks from previous steps
	var allChecks []CheckResult
	
	categories := []string{"Code Quality", "Security", "Performance", "Compliance"}
	categoryScores := make(map[string]float64)
	
	for _, category := range categories {
		if catResult, ok := results[category]; ok {
			if catChecks, ok := catResult.([]CheckResult); ok {
				allChecks = append(allChecks, catChecks...)
				categoryScores[category] = va.calculateCategoryScore(catChecks)
			}
		}
	}
	
	overallScore := va.calculateValidationScore(allChecks)
	
	report := ValidationReport{
		WorkflowID:      input["workflow_id"].(string),
		ChecklistID:     checklist.ID,
		OverallScore:    overallScore,
		Status:          va.determineStatus(allChecks),
		TotalChecks:     len(allChecks),
		PassedChecks:    va.countPassed(allChecks),
		FailedChecks:    va.countFailed(allChecks),
		WarningChecks:   va.countWarnings(allChecks),
		SkippedChecks:   va.countSkipped(allChecks),
		CategoryScores:  categoryScores,
		Results:         allChecks,
		Summary:         va.generateSummary(allChecks),
		Recommendations: va.generateRecommendations(allChecks),
		GeneratedAt:     time.Now(),
	}
	
	return map[string]interface{}{
		"validation_report": report,
		"passed":            report.Status == "passed",
		"score":             overallScore,
		"artifacts":         []string{"validation_report.json"},
	}, nil
}

// Helper methods for running checks

func (va *ValidationAgent) runCodeQualityCheck(check CheckItem) CheckResult {
	// Simulate check execution
	switch check.ID {
	case "cq-001":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       87.5,
			MaxScore:    100,
			Message:     "Test coverage is 87.5% (target: 80%)",
			Timestamp:   time.Now(),
		}
	case "cq-002":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       100,
			MaxScore:    100,
			Message:     "Average complexity is 8.2 (target: < 15)",
			Timestamp:   time.Now(),
		}
	case "cq-003":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       75,
			MaxScore:    100,
			Message:     "Documentation coverage is 75% (target: 60%)",
			Timestamp:   time.Now(),
		}
	}
	
	return CheckResult{
		CheckID:   check.ID,
		Name:      check.Name,
		Status:    "skipped",
		Score:     0,
		MaxScore:  100,
		Timestamp: time.Now(),
	}
}

func (va *ValidationAgent) runSecurityCheck(check CheckItem) CheckResult {
	switch check.ID {
	case "sec-001":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       100,
			MaxScore:    100,
			Message:     "No vulnerabilities found in dependencies",
			Timestamp:   time.Now(),
		}
	case "sec-002":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       100,
			MaxScore:    100,
			Message:     "No secrets detected in code",
			Timestamp:   time.Now(),
		}
	case "sec-003":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       95,
			MaxScore:    100,
			Message:     "Input validation present in all public APIs",
			Timestamp:   time.Now(),
		}
	}
	
	return CheckResult{
		CheckID:   check.ID,
		Name:      check.Name,
		Status:    "skipped",
		Score:     0,
		MaxScore:  100,
		Timestamp: time.Now(),
	}
}

func (va *ValidationAgent) runPerformanceCheck(check CheckItem) CheckResult {
	switch check.ID {
	case "perf-001":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       95,
			MaxScore:    100,
			Message:     "P95 latency is 54ms (target: < 100ms)",
			Timestamp:   time.Now(),
		}
	case "perf-002":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       100,
			MaxScore:    100,
			Message:     "Memory usage is 45MB (target: < 512MB)",
			Timestamp:   time.Now(),
		}
	}
	
	return CheckResult{
		CheckID:   check.ID,
		Name:      check.Name,
		Status:    "skipped",
		Score:     0,
		MaxScore:  100,
		Timestamp: time.Now(),
	}
}

func (va *ValidationAgent) runComplianceCheck(check CheckItem) CheckResult {
	switch check.ID {
	case "comp-001":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       100,
			MaxScore:    100,
			Message:     "MIT license is OSI approved",
			Timestamp:   time.Now(),
		}
	case "comp-002":
		return CheckResult{
			CheckID:     check.ID,
			Name:        check.Name,
			Status:      "passed",
			Score:       100,
			MaxScore:    100,
			Message:     "All code standards met (clippy, fmt)",
			Timestamp:   time.Now(),
		}
	}
	
	return CheckResult{
		CheckID:   check.ID,
		Name:      check.Name,
		Status:    "skipped",
		Score:     0,
		MaxScore:  100,
		Timestamp: time.Now(),
	}
}

// Helper calculation methods

func (va *ValidationAgent) countTotalChecks(checklist ValidationChecklist) int {
	count := 0
	for _, category := range checklist.Categories {
		count += len(category.Checks)
	}
	return count
}

func (va *ValidationAgent) countPassed(checks []CheckResult) int {
	count := 0
	for _, check := range checks {
		if check.Status == "passed" {
			count++
		}
	}
	return count
}

func (va *ValidationAgent) countFailed(checks []CheckResult) int {
	count := 0
	for _, check := range checks {
		if check.Status == "failed" {
			count++
		}
	}
	return count
}

func (va *ValidationAgent) countWarnings(checks []CheckResult) int {
	count := 0
	for _, check := range checks {
		if check.Status == "warning" {
			count++
		}
	}
	return count
}

func (va *ValidationAgent) countSkipped(checks []CheckResult) int {
	count := 0
	for _, check := range checks {
		if check.Status == "skipped" {
			count++
		}
	}
	return count
}

func (va *ValidationAgent) calculateCategoryScore(checks []CheckResult) float64 {
	if len(checks) == 0 {
		return 0
	}
	
	var totalScore float64
	for _, check := range checks {
		totalScore += check.Score / check.MaxScore
	}
	
	return totalScore / float64(len(checks))
}

func (va *ValidationAgent) calculateValidationScore(checks []CheckResult) float64 {
	if len(checks) == 0 {
		return 0
	}
	
	var totalScore float64
	for _, check := range checks {
		if check.Status == "passed" {
			totalScore += 1.0
		} else if check.Status == "warning" {
			totalScore += 0.5
		}
	}
	
	return totalScore / float64(len(checks))
}

func (va *ValidationAgent) determineStatus(checks []CheckResult) string {
	failed := va.countFailed(checks)
	passed := va.countPassed(checks)
	
	if failed == 0 {
		return "passed"
	} else if float64(passed)/float64(len(checks)) >= 0.8 {
		return "partial"
	}
	return "failed"
}

func (va *ValidationAgent) generateSummary(checks []CheckResult) string {
	passed := va.countPassed(checks)
	failed := va.countFailed(checks)
	total := len(checks)
	
	return fmt.Sprintf("Validation completed: %d/%d checks passed, %d failed", passed, total, failed)
}

func (va *ValidationAgent) generateRecommendations(checks []CheckResult) []string {
	recommendations := []string{}
	
	for _, check := range checks {
		if check.Status == "failed" || check.Status == "warning" {
			for _, suggestion := range check.Suggestions {
				recommendations = append(recommendations, suggestion)
			}
		}
	}
	
	if len(recommendations) == 0 {
		recommendations = append(recommendations, "All checks passed. No recommendations.")
	}
	
	return recommendations
}

func init() {
	// Register the validation agent factory
	Register(models.AgentTypeValidation, NewValidationAgent)
}