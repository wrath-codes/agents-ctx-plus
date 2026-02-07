package agents

import (
	"context"
	"fmt"
	"time"

	"github.com/your-org/beads-workflow-system/pkg/models"
)

// POCAgent implements proof-of-concept workflows with saga compensation
type POCAgent struct {
	BaseAgent
}

// NewPOCAgent creates a new POC agent
func NewPOCAgent(agentID string) Agent {
	agent := &POCAgent{
		BaseAgent: BaseAgent{
			ID:   agentID,
			Type: models.AgentTypePOC,
			Capabilities: []string{
				"code_generation",
				"build_automation",
				"test_execution",
				"benchmarking",
				"rollback",
			},
			MaxRetries: 3,
			Timeout:    45 * time.Minute,
		},
	}
	
	agent.Steps = []Step{
		{
			Name:        "setup_environment",
			Description: "Set up development environment",
			Execute:     agent.setupEnvironment,
			Timeout:     5 * time.Minute,
			RetryCount:  2,
		},
		{
			Name:        "generate_implementation",
			Description: "Generate POC implementation code",
			Execute:     agent.generateImplementation,
			Timeout:     10 * time.Minute,
			RetryCount:  2,
		},
		{
			Name:        "build_code",
			Description: "Build the generated code",
			Execute:     agent.buildCode,
			Timeout:     10 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "run_tests",
			Description: "Execute test suite",
			Execute:     agent.runTests,
			Timeout:     10 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "benchmark_performance",
			Description: "Run performance benchmarks",
			Execute:     agent.benchmarkPerformance,
			Timeout:     8 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "cleanup_and_report",
			Description: "Clean up and generate report",
			Execute:     agent.cleanupAndReport,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
	}
	
	return agent
}

// Execute runs the POC workflow with saga compensation
func (pa *POCAgent) Execute(ctx context.Context, workflow *models.Workflow) (*models.Result, error) {
	startTime := time.Now()
	
	// Track saga state for compensation
	sagaState := &SagaState{
		WorkflowID:      workflow.ID,
		StepsCompleted:  []string{},
		Artifacts:       []string{},
		Compensations:   []func() error{},
	}
	
	results := make(map[string]interface{})
	
	// Execute steps with saga compensation
	for i, step := range pa.Steps {
		stepInput := map[string]interface{}{
			"workflow_id": workflow.ID,
			"step_number": i + 1,
			"total_steps": len(pa.Steps),
			"variables":   workflow.Variables,
			"results":     results,
			"saga_state":  sagaState,
		}
		
		stepResult, err := ExecuteStep(ctx, step, stepInput)
		if err != nil {
			// Execute compensations in reverse order
			if compErr := pa.compensate(ctx, sagaState); compErr != nil {
				return nil, fmt.Errorf("step %s failed and compensation failed: %v (original error: %w)", 
					step.Name, compErr, err)
			}
			return nil, fmt.Errorf("step %s failed, compensated: %w", step.Name, err)
		}
		
		// Merge results
		for k, v := range stepResult {
			results[k] = v
		}
		
		// Track completed step for potential compensation
		sagaState.StepsCompleted = append(sagaState.StepsCompleted, step.Name)
		
		// Add compensation function if provided
		if compFn, ok := stepResult["compensation"].(func() error); ok {
			sagaState.Compensations = append(sagaState.Compensations, compFn)
		}
		
		// Track artifacts
		if artifacts, ok := stepResult["artifacts"].([]string); ok {
			sagaState.Artifacts = append(sagaState.Artifacts, artifacts...)
		}
	}
	
	executionTime := time.Since(startTime)
	
	// Calculate success metrics
	buildSuccess, _ := results["build_success"].(bool)
	testSuccess, _ := results["test_success"].(bool)
	
	confidence := pa.calculateConfidence(buildSuccess, testSuccess, results)
	
	return &models.Result{
		WorkflowID:      workflow.ID,
		AgentType:       pa.Type,
		ResultType:      models.ResultTypePOCResults,
		Data:            results,
		ConfidenceScore: confidence,
		QualityScore:    pa.calculateQuality(results),
		ExecutionTimeMs: int(executionTime.Milliseconds()),
		Artifacts:       sagaState.Artifacts,
		CreatedAt:       time.Now(),
	}, nil
}

// SagaState tracks saga execution state
type SagaState struct {
	WorkflowID     string
	StepsCompleted []string
	Artifacts      []string
	Compensations  []func() error
}

// compensate runs compensation functions in reverse order
func (pa *POCAgent) compensate(ctx context.Context, state *SagaState) error {
	for i := len(state.Compensations) - 1; i >= 0; i-- {
		if err := state.Compensations[i](); err != nil {
			// Log compensation failure but continue
			fmt.Printf("Compensation %d failed: %v\n", i, err)
		}
	}
	return nil
}

// POCResult holds POC execution results
type POCResult struct {
	BuildSuccess       bool                   `json:"build_success"`
	TestSuccess        bool                   `json:"test_success"`
	TestResults        TestResults            `json:"test_results"`
	BenchmarkResults   BenchmarkResults       `json:"benchmark_results"`
	PerformanceMetrics PerformanceMetrics     `json:"performance_metrics"`
	CodeMetrics        CodeMetrics            `json:"code_metrics"`
	Artifacts          []string               `json:"artifacts"`
	Errors             []string               `json:"errors,omitempty"`
}

// TestResults holds test execution results
type TestResults struct {
	TotalTests   int      `json:"total_tests"`
	PassedTests  int      `json:"passed_tests"`
	FailedTests  int      `json:"failed_tests"`
	SkippedTests int      `json:"skipped_tests"`
	DurationMs   int      `json:"duration_ms"`
	Coverage     float64  `json:"coverage"`
	TestOutput   []string `json:"test_output"`
}

// BenchmarkResults holds benchmark results
type BenchmarkResults struct {
	Benchmarks    []Benchmark `json:"benchmarks"`
	TotalDuration int         `json:"total_duration_ms"`
}

// Benchmark represents a single benchmark
type Benchmark struct {
	Name         string  `json:"name"`
	Iterations   int     `json:"iterations"`
	DurationNs   int64   `json:"duration_ns_per_op"`
	Throughput   float64 `json:"throughput_ops_per_sec"`
	MemoryBytes  int64   `json:"memory_bytes_per_op"`
	Allocations  int64   `json:"allocations_per_op"`
}

// PerformanceMetrics holds performance metrics
type PerformanceMetrics struct {
	LatencyAvgMs    float64 `json:"latency_avg_ms"`
	LatencyP95Ms    float64 `json:"latency_p95_ms"`
	LatencyP99Ms    float64 `json:"latency_p99_ms"`
	ThroughputRPS   float64 `json:"throughput_rps"`
	MemoryUsageMB   float64 `json:"memory_usage_mb"`
	CPUUsagePercent float64 `json:"cpu_usage_percent"`
}

// CodeMetrics holds code quality metrics
type CodeMetrics struct {
	LinesOfCode     int     `json:"lines_of_code"`
	CyclomaticComplexity float64 `json:"cyclomatic_complexity"`
	MaintainabilityIndex float64 `json:"maintainability_index"`
	TestCoverage    float64 `json:"test_coverage"`
	DocumentationCoverage float64 `json:"documentation_coverage"`
}

// setupEnvironment prepares the development environment
func (pa *POCAgent) setupEnvironment(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	variables, _ := input["variables"].(map[string]interface{})
	language, _ := variables["language"].(string)
	framework, _ := variables["framework"].(string)
	
	if language == "" {
		language = "rust"
	}
	
	// Simulate environment setup
	envSetup := map[string]interface{}{
		"language":         language,
		"framework":        framework,
		"environment_ready": true,
		"tools_installed": []string{
			"cargo",
			"rustc",
			"clippy",
		},
		"workspace_dir": fmt.Sprintf("/tmp/poc-%s", input["workflow_id"]),
	}
	
	return map[string]interface{}{
		"environment":       envSetup,
		"setup_timestamp":   time.Now().Format(time.RFC3339),
		"compensation": func() error {
			// Cleanup workspace on failure
			fmt.Printf("Compensating: cleaning up workspace for %s\n", input["workflow_id"])
			return nil
		},
	}, nil
}

// generateImplementation generates POC code
func (pa *POCAgent) generateImplementation(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	env, _ := results["environment"].(map[string]interface{})
	language, _ := env["language"].(string)
	
	variables, _ := input["variables"].(map[string]interface{})
	feature, _ := variables["feature"].(string)
	
	if feature == "" {
		feature = "basic_implementation"
	}
	
	// Simulate code generation
	generatedFiles := []string{
		"src/main.rs",
		"src/lib.rs",
		"Cargo.toml",
		"tests/integration_tests.rs",
	}
	
	codeMetrics := CodeMetrics{
		LinesOfCode:           150,
		CyclomaticComplexity:  5.2,
		MaintainabilityIndex:  85.0,
		TestCoverage:          0.0, // Will be updated after tests
		DocumentationCoverage: 60.0,
	}
	
	return map[string]interface{}{
		"generated_files": generatedFiles,
		"code_metrics":    codeMetrics,
		"feature":         feature,
		"language":        language,
		"implementation_type": "module",
		"compensation": func() error {
			// Remove generated files on failure
			fmt.Printf("Compensating: removing generated files\n")
			return nil
		},
	}, nil
}

// buildCode compiles the generated code
func (pa *POCAgent) buildCode(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	startTime := time.Now()
	
	// Simulate build process
	buildSuccess := true
	buildOutput := []string{
		"Compiling poc v0.1.0",
		"Finished dev [unoptimized + debuginfo] target(s) in 2.34s",
	}
	
	// Simulate occasional build failure for testing
	// In real implementation, this would actually build the code
	
	duration := time.Since(startTime)
	
	return map[string]interface{}{
		"build_success":  buildSuccess,
		"build_duration_ms": int(duration.Milliseconds()),
		"build_output":   buildOutput,
		"binary_path":    "target/debug/poc",
		"warnings":       2,
		"errors":         0,
	}, nil
}

// runTests executes the test suite
func (pa *POCAgent) runTests(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	startTime := time.Now()
	
	results, _ := input["results"].(map[string]interface{})
	buildSuccess, _ := results["build_success"].(bool)
	
	if !buildSuccess {
		return nil, fmt.Errorf("cannot run tests: build failed")
	}
	
	// Simulate test execution
	testResults := TestResults{
		TotalTests:   15,
		PassedTests:  14,
		FailedTests:  1,
		SkippedTests: 0,
		DurationMs:   2340,
		Coverage:     87.5,
		TestOutput: []string{
			"running 15 tests",
			"test test_basic_functionality ... ok",
			"test test_error_handling ... ok",
			"test test_edge_cases ... FAILED",
			"test result: FAILED. 14 passed; 1 failed",
		},
	}
	
	testSuccess := testResults.FailedTests == 0
	duration := time.Since(startTime)
	
	return map[string]interface{}{
		"test_success":    testSuccess,
		"test_results":    testResults,
		"test_duration_ms": int(duration.Milliseconds()),
	}, nil
}

// benchmarkPerformance runs performance benchmarks
func (pa *POCAgent) benchmarkPerformance(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	testSuccess, _ := results["test_success"].(bool)
	
	if !testSuccess {
		// Still run benchmarks even if some tests failed
		fmt.Println("Warning: Some tests failed, but running benchmarks anyway")
	}
	
	// Simulate benchmark execution
	benchmarks := []Benchmark{
		{
			Name:         "bench_basic_operations",
			Iterations:   100000,
			DurationNs:   5400,
			Throughput:   185185.18,
			MemoryBytes:  128,
			Allocations:  2,
		},
		{
			Name:         "bench_concurrent_access",
			Iterations:   50000,
			DurationNs:   12300,
			Throughput:   81300.81,
			MemoryBytes:  256,
			Allocations:  4,
		},
	}
	
	benchmarkResults := BenchmarkResults{
		Benchmarks:    benchmarks,
		TotalDuration: 5600,
	}
	
	performanceMetrics := PerformanceMetrics{
		LatencyAvgMs:    5.4,
		LatencyP95Ms:    8.2,
		LatencyP99Ms:    12.5,
		ThroughputRPS:   185185,
		MemoryUsageMB:   45.2,
		CPUUsagePercent: 35.8,
	}
	
	return map[string]interface{}{
		"benchmark_results":   benchmarkResults,
		"performance_metrics": performanceMetrics,
		"benchmark_success":   true,
	}, nil
}

// cleanupAndReport generates final report and cleans up
func (pa *POCAgent) cleanupAndReport(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	sagaState, _ := input["saga_state"].(*SagaState)
	
	buildSuccess, _ := results["build_success"].(bool)
	testSuccess, _ := results["test_success"].(bool)
	
	stepsCompleted := 0
	artifactsGenerated := 0
	if sagaState != nil {
		stepsCompleted = len(sagaState.StepsCompleted)
		artifactsGenerated = len(sagaState.Artifacts)
	}
	
	// Generate summary report
	report := map[string]interface{}{
		"build_success":       buildSuccess,
		"test_success":        testSuccess,
		"overall_success":     buildSuccess && testSuccess,
		"steps_completed":     stepsCompleted,
		"total_steps":         len(pa.Steps),
		"artifacts_generated": artifactsGenerated,
		"recommendation":      pa.generateRecommendation(buildSuccess, testSuccess, results),
	}
	
	artifacts := []string{
		"poc_implementation.tar.gz",
		"test_report.json",
		"benchmark_results.json",
		"performance_analysis.md",
	}
	
	return map[string]interface{}{
		"report":    report,
		"artifacts": artifacts,
		"summary":   fmt.Sprintf("POC completed with build=%v, tests=%v", buildSuccess, testSuccess),
	}, nil
}

// Helper methods

func (pa *POCAgent) calculateConfidence(buildSuccess, testSuccess bool, results map[string]interface{}) float64 {
	confidence := 0.5
	
	if buildSuccess {
		confidence += 0.25
	}
	
	if testSuccess {
		confidence += 0.25
	}
	
	// Adjust based on test coverage
	if testResults, ok := results["test_results"].(TestResults); ok {
		coverageBonus := testResults.Coverage / 100.0 * 0.1
		confidence += coverageBonus
	}
	
	if confidence > 1.0 {
		confidence = 1.0
	}
	
	return confidence
}

func (pa *POCAgent) calculateQuality(results map[string]interface{}) float64 {
	quality := 5.0
	
	// Build success adds to quality
	if buildSuccess, ok := results["build_success"].(bool); ok && buildSuccess {
		quality += 2.0
	}
	
	// Test success adds to quality
	if testSuccess, ok := results["test_success"].(bool); ok && testSuccess {
		quality += 2.0
	}
	
	// Test coverage adds bonus
	if testResults, ok := results["test_results"].(TestResults); ok {
		coverageBonus := testResults.Coverage / 100.0
		quality += coverageBonus
	}
	
	if quality > 10.0 {
		quality = 10.0
	}
	
	return quality
}

func (pa *POCAgent) generateRecommendation(buildSuccess, testSuccess bool, results map[string]interface{}) string {
	if buildSuccess && testSuccess {
		return "production_ready"
	} else if buildSuccess && !testSuccess {
		return "needs_testing"
	} else if !buildSuccess {
		return "needs_fixes"
	}
	return "review_required"
}

func init() {
	// Register the POC agent factory
	Register(models.AgentTypePOC, NewPOCAgent)
}