package agents

import (
	"context"
	"encoding/json"
	"fmt"
	"os/exec"
	"strings"
	"time"

	"github.com/your-org/beads-workflow-system/internal/llm"
	"github.com/your-org/beads-workflow-system/internal/registry"
	"github.com/your-org/beads-workflow-system/pkg/models"
)

// ResearchAgent performs research on libraries, frameworks, and technologies.
// It uses real package registry APIs and an LLM for analysis and synthesis.
type ResearchAgent struct {
	BaseAgent
	llm      llm.Provider
	registry *registry.Client
}

// NewResearchAgent creates a new research agent with simulated LLM (for backward compat).
// Use NewResearchAgentWithLLM for real functionality.
func NewResearchAgent(agentID string) Agent {
	return newResearchAgent(agentID, nil)
}

// NewResearchAgentWithLLM creates a research agent backed by a real LLM provider.
func NewResearchAgentWithLLM(agentID string, provider llm.Provider) Agent {
	return newResearchAgent(agentID, provider)
}

func newResearchAgent(agentID string, provider llm.Provider) Agent {
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
		llm:      provider,
		registry: registry.NewClient(),
	}

	agent.Steps = []Step{
		{
			Name:        "library_discovery",
			Description: "Discover and identify relevant libraries from package registries",
			Execute:     agent.discoverLibraries,
			Timeout:     5 * time.Minute,
			RetryCount:  2,
		},
		{
			Name:        "documentation_analysis",
			Description: "Fetch and analyze library documentation via LLM",
			Execute:     agent.analyzeDocumentation,
			Timeout:     10 * time.Minute,
			RetryCount:  2,
		},
		{
			Name:        "static_analysis",
			Description: "Analyze local project context and compatibility",
			Execute:     agent.performStaticAnalysis,
			Timeout:     8 * time.Minute,
			RetryCount:  1,
		},
		{
			Name:        "findings_synthesis",
			Description: "Synthesize findings and generate recommendations via LLM",
			Execute:     agent.synthesizeFindings,
			Timeout:     5 * time.Minute,
			RetryCount:  1,
		},
	}

	return agent
}

// Execute runs the research workflow.
func (ra *ResearchAgent) Execute(ctx context.Context, workflow *models.Workflow) (*models.Result, error) {
	startTime := time.Now()

	results := make(map[string]interface{})
	var findings []LibraryFinding

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

		for k, v := range stepResult {
			results[k] = v
		}

		if step.Name == "library_discovery" {
			if libs, ok := stepResult["libraries"].([]LibraryFinding); ok {
				findings = libs
			}
		}
	}

	executionTime := time.Since(startTime)
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

// LibraryFinding represents a discovered library with analysis.
type LibraryFinding struct {
	Name             string                 `json:"name"`
	Version          string                 `json:"version"`
	Description      string                 `json:"description"`
	DocumentationURL string                 `json:"documentation_url"`
	RepositoryURL    string                 `json:"repository_url"`
	Stars            int                    `json:"stars"`
	License          string                 `json:"license"`
	LastUpdated      time.Time              `json:"last_updated"`
	RelevanceScore   float64                `json:"relevance_score"`
	ConfidenceScore  float64                `json:"confidence_score"`
	Pros             []string               `json:"pros"`
	Cons             []string               `json:"cons"`
	Metrics          map[string]interface{} `json:"metrics"`
	Registry         string                 `json:"registry"`
	Downloads        int64                  `json:"downloads"`
	ReadmeURL        string                 `json:"readme_url,omitempty"`
}

// -----------------------------------------------------------------------
// Step 1: Library Discovery — queries real package registries
// -----------------------------------------------------------------------

func (ra *ResearchAgent) discoverLibraries(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	variables, _ := input["variables"].(map[string]interface{})
	query, _ := variables["query"].(string)
	focus, _ := variables["focus"].(string)

	if query == "" {
		return nil, fmt.Errorf("research query is required (set variables.query)")
	}

	// Determine which registries to search based on focus/ecosystem.
	ecosystem, _ := variables["ecosystem"].(string)
	limit := 5

	var allFindings []LibraryFinding

	switch strings.ToLower(ecosystem) {
	case "rust", "cargo":
		sr, err := ra.registry.SearchCratesIO(ctx, query, limit)
		if err != nil {
			return nil, fmt.Errorf("crates.io search failed: %w", err)
		}
		allFindings = registryResultToFindings(sr)

	case "node", "npm", "javascript", "typescript":
		sr, err := ra.registry.SearchNPM(ctx, query, limit)
		if err != nil {
			return nil, fmt.Errorf("npm search failed: %w", err)
		}
		allFindings = registryResultToFindings(sr)

	case "elixir", "erlang", "hex":
		sr, err := ra.registry.SearchHex(ctx, query, limit)
		if err != nil {
			return nil, fmt.Errorf("hex search failed: %w", err)
		}
		allFindings = registryResultToFindings(sr)

	case "python", "pypi":
		sr, err := ra.registry.SearchPyPI(ctx, query, limit)
		if err != nil {
			return nil, fmt.Errorf("pypi search failed: %w", err)
		}
		allFindings = registryResultToFindings(sr)

	default:
		// Search all registries concurrently.
		results, err := ra.registry.SearchAll(ctx, query, limit)
		if err != nil {
			return nil, fmt.Errorf("multi-registry search failed: %w", err)
		}
		for _, sr := range results {
			allFindings = append(allFindings, registryResultToFindings(&sr)...)
		}
	}

	return map[string]interface{}{
		"libraries":       allFindings,
		"query":           query,
		"focus":           focus,
		"ecosystem":       ecosystem,
		"libraries_found": len(allFindings),
	}, nil
}

// registryResultToFindings converts registry search results to LibraryFindings.
func registryResultToFindings(sr *registry.SearchResult) []LibraryFinding {
	var findings []LibraryFinding
	for _, pkg := range sr.Packages {
		findings = append(findings, LibraryFinding{
			Name:             pkg.Name,
			Version:          pkg.Version,
			Description:      pkg.Description,
			DocumentationURL: pkg.DocumentsURL,
			RepositoryURL:    pkg.Repository,
			License:          pkg.License,
			Downloads:        pkg.Downloads,
			Registry:         pkg.Registry,
			ReadmeURL:        pkg.ReadmeURL,
			RelevanceScore:   0.5, // will be refined by LLM
			ConfidenceScore:  0.5,
			Metrics: map[string]interface{}{
				"downloads": pkg.Downloads,
				"registry":  pkg.Registry,
			},
		})
	}
	return findings
}

// -----------------------------------------------------------------------
// Step 2: Documentation Analysis — fetches READMEs, asks LLM to analyze
// -----------------------------------------------------------------------

func (ra *ResearchAgent) analyzeDocumentation(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	findings, _ := results["libraries"].([]LibraryFinding)
	query, _ := results["query"].(string)
	focus, _ := results["focus"].(string)

	if ra.llm == nil {
		// Fallback: return basic analysis without LLM.
		return ra.analyzeDocumentationFallback(findings)
	}

	docAnalysis := make(map[string]interface{})

	for _, lib := range findings {
		// Fetch README if available.
		var readme string
		if lib.ReadmeURL != "" {
			content, err := ra.registry.FetchReadme(ctx, lib.ReadmeURL)
			if err == nil {
				readme = content
			}
		}

		// Ask LLM to analyze this library.
		prompt := fmt.Sprintf(`You are a senior software engineer evaluating libraries.

Analyze the following library for the research query: "%s" (focus: %s)

Library: %s v%s
Registry: %s
Description: %s
Downloads: %d
License: %s
Docs: %s

%s

Respond in JSON with exactly these fields:
{
  "relevance_score": 0.0-1.0 (how relevant to the query),
  "quality_score": 0.0-1.0 (overall quality assessment),
  "pros": ["list", "of", "strengths"],
  "cons": ["list", "of", "weaknesses"],
  "api_coverage": "comprehensive|good|basic|poor",
  "documentation_quality": "excellent|good|fair|poor",
  "maturity": "mature|growing|new|abandoned",
  "summary": "1-2 sentence summary of the library's value for this use case"
}

Only respond with valid JSON, no markdown fences.`,
			query, focus,
			lib.Name, lib.Version,
			lib.Registry,
			lib.Description,
			lib.Downloads,
			lib.License,
			lib.DocumentationURL,
			readmeSection(readme),
		)

		resp, err := ra.llm.Complete(ctx, llm.CompletionRequest{
			Messages: []llm.Message{
				llm.UserMessage(prompt),
			},
			Temperature: 0.3,
			MaxTokens:   1024,
		})
		if err != nil {
			// Best-effort: log and continue.
			docAnalysis[lib.Name] = map[string]interface{}{
				"error": err.Error(),
			}
			continue
		}

		// Parse the LLM response.
		var analysis map[string]interface{}
		if err := json.Unmarshal([]byte(resp.Content), &analysis); err != nil {
			// Try to extract JSON from response if it has markdown fences.
			cleaned := extractJSON(resp.Content)
			if err2 := json.Unmarshal([]byte(cleaned), &analysis); err2 != nil {
				docAnalysis[lib.Name] = map[string]interface{}{
					"raw_response": resp.Content,
					"parse_error":  err.Error(),
				}
				continue
			}
		}
		docAnalysis[lib.Name] = analysis
	}

	return map[string]interface{}{
		"documentation_analysis": docAnalysis,
		"libraries_analyzed":     len(findings),
	}, nil
}

func (ra *ResearchAgent) analyzeDocumentationFallback(findings []LibraryFinding) (map[string]interface{}, error) {
	docAnalysis := make(map[string]interface{})
	for _, lib := range findings {
		docAnalysis[lib.Name] = map[string]interface{}{
			"documentation_url": lib.DocumentationURL,
			"note":              "LLM not available; manual review recommended",
		}
	}
	return map[string]interface{}{
		"documentation_analysis": docAnalysis,
		"libraries_analyzed":     len(findings),
	}, nil
}

// -----------------------------------------------------------------------
// Step 3: Static Analysis — reads local project, runs shell commands
// -----------------------------------------------------------------------

func (ra *ResearchAgent) performStaticAnalysis(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	variables, _ := input["variables"].(map[string]interface{})
	projectPath, _ := variables["project_path"].(string)

	analysis := make(map[string]interface{})

	// Try to detect project type and run relevant commands.
	if projectPath != "" {
		analysis["project_path"] = projectPath

		// Detect Go project.
		if out, err := runCmd(ctx, projectPath, "go", "list", "-m", "-json"); err == nil {
			analysis["go_module"] = string(out)
		}

		// Detect Rust project.
		if out, err := runCmd(ctx, projectPath, "cargo", "metadata", "--format-version=1", "--no-deps"); err == nil {
			// Just grab the package names.
			var meta struct {
				Packages []struct {
					Name    string `json:"name"`
					Version string `json:"version"`
				} `json:"packages"`
			}
			if json.Unmarshal(out, &meta) == nil {
				analysis["cargo_packages"] = meta.Packages
			}
		}

		// Detect Node project.
		if out, err := runCmd(ctx, projectPath, "node", "-e", "console.log(JSON.stringify(require('./package.json')))"); err == nil {
			var pkg map[string]interface{}
			if json.Unmarshal(out, &pkg) == nil {
				analysis["npm_package"] = map[string]interface{}{
					"name":         pkg["name"],
					"version":      pkg["version"],
					"dependencies": pkg["dependencies"],
				}
			}
		}
	}

	// If no project path, just report that static analysis was skipped.
	if len(analysis) == 0 {
		analysis["note"] = "No project_path provided; static analysis skipped. Set variables.project_path to enable."
	}

	return map[string]interface{}{
		"static_analysis": analysis,
		"analysis_method": "automated",
	}, nil
}

// -----------------------------------------------------------------------
// Step 4: Synthesis — LLM generates final research report
// -----------------------------------------------------------------------

func (ra *ResearchAgent) synthesizeFindings(ctx context.Context, input map[string]interface{}) (map[string]interface{}, error) {
	results, _ := input["results"].(map[string]interface{})
	findings, _ := results["libraries"].([]LibraryFinding)
	docAnalysis, _ := results["documentation_analysis"].(map[string]interface{})
	staticAnalysis, _ := results["static_analysis"].(map[string]interface{})
	query, _ := results["query"].(string)
	focus, _ := results["focus"].(string)

	if ra.llm == nil {
		return ra.synthesizeFallback(findings, docAnalysis, staticAnalysis, query, focus)
	}

	// Build a summary of all findings for the LLM.
	findingsJSON, _ := json.MarshalIndent(map[string]interface{}{
		"query":          query,
		"focus":          focus,
		"libraries":      findings,
		"doc_analysis":   docAnalysis,
		"static_context": staticAnalysis,
	}, "", "  ")

	prompt := fmt.Sprintf(`You are a senior technical researcher. Based on the following research data, write a comprehensive research report.

Research Data:
%s

Write a report in JSON with exactly these fields:
{
  "executive_summary": "2-3 sentences summarizing the research",
  "primary_recommendation": "name of the top recommended library",
  "recommendation_reasoning": "why this library is the best choice",
  "risk_assessment": "low|medium|high",
  "ranked_libraries": [
    {
      "name": "library name",
      "rank": 1,
      "recommendation": "highly_recommended|recommended|consider|not_recommended",
      "relevance_score": 0.0-1.0,
      "confidence_score": 0.0-1.0,
      "pros": ["..."],
      "cons": ["..."],
      "best_for": "brief description of ideal use case"
    }
  ],
  "next_steps": ["actionable next steps for the team"],
  "caveats": ["any important caveats or limitations"]
}

Only respond with valid JSON, no markdown fences.`, string(findingsJSON))

	resp, err := ra.llm.Complete(ctx, llm.CompletionRequest{
		Messages: []llm.Message{
			llm.UserMessage(prompt),
		},
		Temperature: 0.4,
		MaxTokens:   2048,
	})
	if err != nil {
		// Fallback to basic synthesis.
		return ra.synthesizeFallback(findings, docAnalysis, staticAnalysis, query, focus)
	}

	var synthesis map[string]interface{}
	if err := json.Unmarshal([]byte(resp.Content), &synthesis); err != nil {
		cleaned := extractJSON(resp.Content)
		if err2 := json.Unmarshal([]byte(cleaned), &synthesis); err2 != nil {
			synthesis = map[string]interface{}{
				"raw_response": resp.Content,
				"parse_error":  err.Error(),
			}
		}
	}

	// Update finding scores from LLM analysis if available.
	if ranked, ok := synthesis["ranked_libraries"].([]interface{}); ok {
		for _, r := range ranked {
			if rm, ok := r.(map[string]interface{}); ok {
				name, _ := rm["name"].(string)
				for i := range findings {
					if strings.EqualFold(findings[i].Name, name) {
						if rel, ok := rm["relevance_score"].(float64); ok {
							findings[i].RelevanceScore = rel
						}
						if conf, ok := rm["confidence_score"].(float64); ok {
							findings[i].ConfidenceScore = conf
						}
						if pros, ok := rm["pros"].([]interface{}); ok {
							findings[i].Pros = toStringSlice(pros)
						}
						if cons, ok := rm["cons"].([]interface{}); ok {
							findings[i].Cons = toStringSlice(cons)
						}
					}
				}
			}
		}
	}

	return map[string]interface{}{
		"synthesis":       synthesis,
		"recommendations": synthesis["ranked_libraries"],
		"summary": map[string]interface{}{
			"primary_recommendation": synthesis["primary_recommendation"],
			"reasoning":              synthesis["recommendation_reasoning"],
			"risk_assessment":        synthesis["risk_assessment"],
			"executive_summary":      synthesis["executive_summary"],
		},
		"analysis_summary": ra.generateAnalysisSummary(findings),
	}, nil
}

func (ra *ResearchAgent) synthesizeFallback(findings []LibraryFinding, docAnalysis, staticAnalysis map[string]interface{}, query, focus string) (map[string]interface{}, error) {
	var recommendations []map[string]interface{}
	for _, lib := range findings {
		recommendations = append(recommendations, map[string]interface{}{
			"library":    lib.Name,
			"version":    lib.Version,
			"registry":   lib.Registry,
			"downloads":  lib.Downloads,
			"confidence": lib.ConfidenceScore,
		})
	}

	summary := map[string]interface{}{
		"primary_recommendation": "",
		"reasoning":              "LLM not available; showing raw registry data",
		"risk_assessment":        "unknown",
	}
	if len(findings) > 0 {
		// Pick the one with the most downloads.
		top := findings[0]
		for _, f := range findings[1:] {
			if f.Downloads > top.Downloads {
				top = f
			}
		}
		summary["primary_recommendation"] = top.Name
	}

	return map[string]interface{}{
		"recommendations":  recommendations,
		"summary":          summary,
		"analysis_summary": ra.generateAnalysisSummary(findings),
	}, nil
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

func readmeSection(readme string) string {
	if readme == "" {
		return ""
	}
	return fmt.Sprintf("README (first 8KB):\n%s", readme)
}

func extractJSON(s string) string {
	// Try to find JSON between ```json and ``` fences.
	if idx := strings.Index(s, "```json"); idx != -1 {
		s = s[idx+7:]
	} else if idx := strings.Index(s, "```"); idx != -1 {
		s = s[idx+3:]
	}
	if idx := strings.LastIndex(s, "```"); idx != -1 {
		s = s[:idx]
	}
	return strings.TrimSpace(s)
}

func toStringSlice(items []interface{}) []string {
	var result []string
	for _, item := range items {
		if s, ok := item.(string); ok {
			result = append(result, s)
		}
	}
	return result
}

func runCmd(ctx context.Context, dir string, name string, args ...string) ([]byte, error) {
	cmd := exec.CommandContext(ctx, name, args...)
	cmd.Dir = dir
	return cmd.Output()
}

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
	if len(findings) > 1 {
		avgConfidence = minF(avgConfidence+0.05, 1.0)
	}
	return avgConfidence
}

func (ra *ResearchAgent) calculateQuality(findings []LibraryFinding) float64 {
	if len(findings) == 0 {
		return 0.0
	}
	baseQuality := 7.0
	if len(findings) > 1 {
		baseQuality += 1.0
	}
	for _, f := range findings {
		if f.ConfidenceScore > 0.9 {
			baseQuality += 0.5
		}
	}
	return minF(baseQuality, 10.0)
}

func (ra *ResearchAgent) generateArtifacts(results map[string]interface{}) []string {
	return []string{
		"research_findings.json",
		"library_comparison.md",
		"recommendations.md",
	}
}

func (ra *ResearchAgent) generateAnalysisSummary(findings []LibraryFinding) map[string]interface{} {
	registries := make(map[string]bool)
	for _, f := range findings {
		if f.Registry != "" {
			registries[f.Registry] = true
		}
	}
	var sources []string
	for r := range registries {
		sources = append(sources, r)
	}
	if len(sources) == 0 {
		sources = []string{"none"}
	}

	return map[string]interface{}{
		"total_libraries_found": len(findings),
		"analysis_date":         time.Now().Format("2006-01-02"),
		"methodology":           "automated_research",
		"sources":               sources,
	}
}

func minF(a, b float64) float64 {
	if a < b {
		return a
	}
	return b
}

func init() {
	Register(models.AgentTypeResearch, NewResearchAgent)
}
