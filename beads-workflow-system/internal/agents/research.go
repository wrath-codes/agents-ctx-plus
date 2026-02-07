package agents

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/your-org/beads-workflow-system/pkg/models"
)

// ResearchAgent performs research on libraries, frameworks, and technologies
type ResearchAgent struct {
	BaseAgent
}

// NewResearchAgent creates a new research agent
func NewResearchAgent(agentID string) Agent {
	agent := &ResearchAgent{
		BaseAgent: BaseAgent{
			ID:   agentID,
			Type: models.AgentTypeResearch,
			Capabilities: []string{
				"library_discovery",
				"documentation_analysis",
				"static_analysis",
				"benchmark_comparison",
			},
			MaxRetries: 3,
			Timeout:    30 * time.Minute,
		},
	}
	
	agent.Steps = []Step{
		{
			Name:        "library_discovery",
			Description: "Discover and identify relevant libraries",
			Execute:     agent.discoverLibraries,
			Timeout:     5 * time.Minute,
			RetryCount:  2,
		},
		{
			Name:        "documentation_analysis",
			Description: "Analyze library documentation and APIs",
			Execute:     agent.analyzeDocumentation,
			Timeout:     10 * time.Minute,
			RetryCount:  2,
		},
		{
			Name:        "static_analysis",
			Description: "Perform static code analysis",
			Execute:     agent.performStaticAnalysis,
			Timeout:     8 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "findings_synthesis",
			Description: "Synthesize findings and generate recommendations",
			Execute:     agent.synthesizeFindings,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
	}
	
	return agent
}

// Execute runs the research workflow
func (ra *ResearchAgent) Execute(ctx context.Context, workflow *models.Workflow) (*models.Result, error) {
	startTime := time.Now()
	
	results := make(map[string]interface{})
	var findings []LibraryFinding
	
	// Execute each step in sequence
	for i, step := range ra.Steps {
		stepInput := map[string]interface{}{
			"workflow_id": workflow.ID,
			"step_number": i + 1,
			"total_steps": len(ra.Steps),
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
		
		// Collect findings from library discovery
		if step.Name == "library_discovery" {
			if libs, ok := stepResult["libraries"].([]LibraryFinding); ok {
				findings = libs
			}
		}
	}
	
	executionTime := time.Since(startTime)
	
	// Calculate confidence score based on findings quality
	confidence := ra.calculateConfidence(findings)
	
	return &models.Result{
		WorkflowID:      workflow.ID,
		AgentType:       ra.Type,
		ResultType:      models.ResultTypeFindings,
		Data:            results,
		ConfidenceScore: confidence,
		QualityScore:    ra.calculateQuality(findings),
		ExecutionTimeMs: int(executionTime.Milliseconds()),
		Artifacts:       ra.generateArtifacts(results),
		CreatedAt:       time.Now(),
	}, nil
}

// LibraryFinding represents a discovered library with analysis
type LibraryFinding struct {
	Name              string                 `json:"name"`
	Version           string                 `json:"version"`
	Description       string                 `json:"description"`
	DocumentationURL  string                 `json:"documentation_url"`
	RepositoryURL     string                 `json:"repository_url"`
	Stars             int                    `json:"stars"`
	License           string                 `json:"license"`
	LastUpdated       time.Time              `json:"last_updated"`
	RelevanceScore    float64                `json:"relevance_score"`
	ConfidenceScore   float64                `json:"confidence_score"`
	Pros              []string               `json:"pros"`
	Cons              []string               `json:"cons"`
	Metrics           map[string]interface{} `json:"metrics"`
}

// discoverLibraries identifies relevant libraries based on research query
func (ra *ResearchAgent) discoverLibraries(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	variables, _ := input["variables"].(map[string]interface{})
	query, _ := variables["query"].(string)
	focus, _ := variables["focus"].(string)
	
	// Simulate library discovery
	// In a real implementation, this would search package registries, GitHub, etc.
	libraries := []LibraryFinding{
		{
			Name:             "tokio",
			Version:          "1.35.0",
			Description:      "A runtime for writing reliable asynchronous applications",
			DocumentationURL: "https://docs.rs/tokio",
			RepositoryURL:    "https://github.com/tokio-rs/tokio",
			Stars:            26000,
			License:          "MIT",
			LastUpdated:      time.Now().Add(-30 * 24 * time.Hour),
			RelevanceScore:   0.95,
			ConfidenceScore:  0.92,
			Pros: []string{
				"Excellent performance",
				"Large ecosystem",
				"Battle-tested in production",
				"Active maintenance",
			},
			Cons: []string{
				"Steep learning curve",
				"Complex for simple use cases",
			},
			Metrics: map[string]interface{}{
				"downloads_per_month": 15000000,
				"github_stars":        26000,
				"maintenance_score":   0.95,
			},
		},
		{
			Name:             "async-std",
			Version:          "1.12.0",
			Description:      "Async version of the Rust standard library",
			DocumentationURL: "https://docs.rs/async-std",
			RepositoryURL:    "https://github.com/async-rs/async-std",
			Stars:            4200,
			License:          "Apache-2.0/MIT",
			LastUpdated:      time.Now().Add(-60 * 24 * time.Hour),
			RelevanceScore:   0.85,
			ConfidenceScore:  0.88,
			Pros: []string{
				"Standard library API compatibility",
				"Simpler than tokio",
			},
			Cons: []string{
				"Smaller ecosystem",
				"Less active development",
			},
			Metrics: map[string]interface{}{
				"downloads_per_month": 800000,
				"github_stars":        4200,
				"maintenance_score":   0.75,
			},
		},
	}
	
	// If specific libraries were requested, filter or prioritize them
	if requestedLibs, ok := variables["libraries"].(string); ok && requestedLibs != "" {
		requested := strings.Split(requestedLibs, ",")
		for i := range requested {
			requested[i] = strings.TrimSpace(requested[i])
		}
		
		// Boost relevance for requested libraries
		for i := range libraries {
			for _, req := range requested {
				if strings.EqualFold(libraries[i].Name, req) {
					libraries[i].RelevanceScore = 1.0
				}
			}
		}
	}
	
	return map[string]interface{}{
		"libraries":       libraries,
		"query":           query,
		"focus":           focus,
		"libraries_found": len(libraries),
	}, nil
}

// analyzeDocumentation analyzes library documentation
func (ra *ResearchAgent) analyzeDocumentation(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	librariesRaw, _ := results["libraries"].([]LibraryFinding)
	
	docAnalysis := make(map[string]interface{})
	
	for _, lib := range librariesRaw {
		analysis := map[string]interface{}{
			"api_coverage":       "comprehensive",
			"examples_quality":   "excellent",
			"getting_started":    "well_documented",
			"advanced_topics":    "good",
			"community_support":  communitySupport(lib.Stars),
			"documentation_url":  lib.DocumentationURL,
		}
		
		docAnalysis[lib.Name] = analysis
	}
	
	return map[string]interface{}{
		"documentation_analysis": docAnalysis,
		"libraries_analyzed":     len(librariesRaw),
	}, nil
}

// performStaticAnalysis performs code analysis (simulated)
func (ra *ResearchAgent) performStaticAnalysis(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	librariesRaw, _ := results["libraries"].([]LibraryFinding)
	
	analysis := make(map[string]interface{})
	
	for _, lib := range librariesRaw {
		// Simulated static analysis metrics
		libAnalysis := map[string]interface{}{
			"code_quality_score":   0.88,
			"test_coverage":        "85%",
			"security_score":       0.92,
			"complexity_score":     "moderate",
			"dependencies":         12,
			"transitive_deps":      45,
			"binary_size_impact":   "medium",
			"compile_time_impact":  "low",
		}
		
		analysis[lib.Name] = libAnalysis
	}
	
	return map[string]interface{}{
		"static_analysis":      analysis,
		"analysis_method":      "automated",
		"confidence":           0.85,
	}, nil
}

// synthesizeFindings generates final recommendations
func (ra *ResearchAgent) synthesizeFindings(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	librariesRaw, _ := results["libraries"].([]LibraryFinding)
	docAnalysis, _ := results["documentation_analysis"].(map[string]interface{})
	staticAnalysis, _ := results["static_analysis"].(map[string]interface{})
	
	// Generate recommendations
	var recommendations []map[string]interface{}
	
	for _, lib := range librariesRaw {
		rec := map[string]interface{}{
			"library":         lib.Name,
			"recommendation":  ra.getRecommendation(lib),
			"confidence":      lib.ConfidenceScore,
			"use_cases":       ra.getUseCases(lib),
			"alternatives":    []string{},
		}
		
		if doc, ok := docAnalysis[lib.Name]; ok {
			rec["documentation_quality"] = doc
		}
		
		if static, ok := staticAnalysis[lib.Name]; ok {
			rec["code_quality"] = static
		}
		
		recommendations = append(recommendations, rec)
	}
	
	// Generate final summary
	summary := map[string]interface{}{
		"primary_recommendation": "",
		"reasoning":              "",
		"risk_assessment":        "low",
	}
	
	if len(librariesRaw) > 0 {
		// Sort by confidence and recommend top one
		topLib := librariesRaw[0]
		for _, lib := range librariesRaw {
			if lib.ConfidenceScore > topLib.ConfidenceScore {
				topLib = lib
			}
		}
		summary["primary_recommendation"] = topLib.Name
		summary["reasoning"] = fmt.Sprintf("Selected based on %s performance in %s", topLib.Name, input["focus"])
	}
	
	return map[string]interface{}{
		"recommendations":  recommendations,
		"summary":          summary,
		"analysis_summary": ra.generateAnalysisSummary(librariesRaw),
	}, nil
}

// Helper methods

func communitySupport(stars int) string {
	if stars > 10000 {
		return "strong"
	}
	return "moderate"
}

func (ra *ResearchAgent) calculateConfidence(findings []LibraryFinding) float64 {
	if len(findings) == 0 {
		return 0.0
	}
	
	var totalConfidence float64
	for _, f := range findings {
		totalConfidence += f.ConfidenceScore
	}
	
	avgConfidence := totalConfidence / float64(len(findings))
	
	// Boost confidence if we have multiple libraries to compare
	if len(findings) > 1 {
		avgConfidence = min(avgConfidence+0.05, 1.0)
	}
	
	return avgConfidence
}

func (ra *ResearchAgent) calculateQuality(findings []LibraryFinding) float64 {
	if len(findings) == 0 {
		return 0.0
	}
	
	// Quality based on depth of findings
	baseQuality := 7.0
	
	// Bonus for multiple libraries
	if len(findings) > 1 {
		baseQuality += 1.0
	}
	
	// Bonus for high confidence
	for _, f := range findings {
		if f.ConfidenceScore > 0.9 {
			baseQuality += 0.5
		}
	}
	
	return min(baseQuality, 10.0)
}

func (ra *ResearchAgent) generateArtifacts(results map[string]interface{}) []string {
	return []string{
		"research_findings.json",
		"library_comparison.md",
		"recommendations.md",
	}
}

func (ra *ResearchAgent) getRecommendation(lib LibraryFinding) string {
	if lib.ConfidenceScore > 0.9 && lib.RelevanceScore > 0.9 {
		return "highly_recommended"
	} else if lib.ConfidenceScore > 0.8 {
		return "recommended"
	} else if lib.ConfidenceScore > 0.6 {
		return "consider"
	}
	return "not_recommended"
}

func (ra *ResearchAgent) getUseCases(lib LibraryFinding) []string {
	useCases := []string{
		"production_applications",
		"high_performance_services",
	}
	
	if lib.Stars > 10000 {
		useCases = append(useCases, "enterprise_use")
	}
	
	return useCases
}

func (ra *ResearchAgent) generateAnalysisSummary(findings []LibraryFinding) map[string]interface{} {
	return map[string]interface{}{
		"total_libraries_found": len(findings),
		"analysis_date":         time.Now().Format("2006-01-02"),
		"methodology":           "automated_research",
		"sources": []string{
			"crates.io",
			"github",
			"docs.rs",
		},
	}
}

func min(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}

func init() {
	// Register the research agent factory
	Register(models.AgentTypeResearch, NewResearchAgent)
}