package monitoring

import (
	"context"
	"database/sql"
	"fmt"
	"time"
)

// HandoffOptimizer handles intelligent agent selection and handoff
type HandoffOptimizer struct {
	db *sql.DB
}

// NewHandoffOptimizer creates a new handoff optimizer
func NewHandoffOptimizer(db *sql.DB) *HandoffOptimizer {
	return &HandoffOptimizer{db: db}
}

// AgentScore represents a candidate agent with a computed score
type AgentScore struct {
	AgentID         string  `json:"agent_id"`
	AgentType       string  `json:"agent_type"`
	Score           float64 `json:"score"`
	CurrentWorkload int     `json:"current_workload"`
	MaxWorkload     int     `json:"max_workload"`
	AvailableSlots  int     `json:"available_slots"`
	SuccessRate     float64 `json:"success_rate"`
	AvgDurationMs   float64 `json:"avg_duration_ms"`
	Reason          string  `json:"reason"`
}

// SelectBestAgent selects the best available agent for a workflow
func (ho *HandoffOptimizer) SelectBestAgent(ctx context.Context, workflowType string) (*AgentScore, error) {
	candidates, err := ho.getCandidates(ctx, workflowType)
	if err != nil {
		return nil, err
	}

	if len(candidates) == 0 {
		return nil, fmt.Errorf("no available agents for workflow type: %s", workflowType)
	}

	// Score each candidate
	for i := range candidates {
		candidates[i].Score = ho.scoreAgent(candidates[i])
	}

	// Select highest scoring agent
	best := candidates[0]
	for _, c := range candidates[1:] {
		if c.Score > best.Score {
			best = c
		}
	}

	best.Reason = ho.explainSelection(best)
	return &best, nil
}

// getCandidates retrieves all agents that could handle the workflow type
func (ho *HandoffOptimizer) getCandidates(ctx context.Context, workflowType string) ([]AgentScore, error) {
	rows, err := ho.db.QueryContext(ctx, `
		SELECT 
			ac.agent_id,
			ac.agent_type,
			ac.current_workload,
			ac.max_workload,
			ac.max_workload - ac.current_workload as available_slots
		FROM agent_configurations ac
		WHERE ac.agent_type = ? 
		  AND ac.status = 'active'
		  AND ac.current_workload < ac.max_workload
		ORDER BY ac.current_workload ASC
	`, workflowType)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var candidates []AgentScore
	for rows.Next() {
		var c AgentScore
		if err := rows.Scan(&c.AgentID, &c.AgentType, &c.CurrentWorkload, &c.MaxWorkload, &c.AvailableSlots); err != nil {
			continue
		}

		// Fetch historical success rate
		c.SuccessRate = ho.getAgentSuccessRate(ctx, c.AgentID)
		c.AvgDurationMs = ho.getAgentAvgDuration(ctx, c.AgentID)

		candidates = append(candidates, c)
	}

	return candidates, rows.Err()
}

// scoreAgent computes a composite score for agent selection
// Higher is better. Weighted factors:
//   - Availability (40%): fewer current tasks = better
//   - Success rate (35%): higher historical success = better
//   - Speed (25%): lower average duration = better
func (ho *HandoffOptimizer) scoreAgent(agent AgentScore) float64 {
	// Availability: ratio of free slots
	availabilityScore := 0.0
	if agent.MaxWorkload > 0 {
		availabilityScore = float64(agent.AvailableSlots) / float64(agent.MaxWorkload)
	}

	// Success rate: already 0-1
	successScore := agent.SuccessRate / 100.0

	// Speed: normalize inverse duration (cap at 10 minutes = 600000ms)
	speedScore := 1.0
	if agent.AvgDurationMs > 0 {
		speedScore = 1.0 - (agent.AvgDurationMs / 600000.0)
		if speedScore < 0 {
			speedScore = 0
		}
	}

	return (availabilityScore * 0.40) + (successScore * 0.35) + (speedScore * 0.25)
}

// explainSelection provides a human-readable explanation for the selection
func (ho *HandoffOptimizer) explainSelection(agent AgentScore) string {
	return fmt.Sprintf(
		"selected %s (score=%.2f): %d/%d slots used, %.0f%% success rate, avg %.0fms",
		agent.AgentID, agent.Score,
		agent.CurrentWorkload, agent.MaxWorkload,
		agent.SuccessRate, agent.AvgDurationMs,
	)
}

// getAgentSuccessRate returns the historical success rate for an agent
func (ho *HandoffOptimizer) getAgentSuccessRate(ctx context.Context, agentID string) float64 {
	var total, completed int
	err := ho.db.QueryRowContext(ctx, `
		SELECT COUNT(*), SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END)
		FROM agent_assignments WHERE agent_id = ?
	`, agentID).Scan(&total, &completed)

	if err != nil || total == 0 {
		return 100.0 // Default: assume perfect if no history
	}

	return float64(completed) / float64(total) * 100
}

// getAgentAvgDuration returns the average workflow duration for an agent
func (ho *HandoffOptimizer) getAgentAvgDuration(ctx context.Context, agentID string) float64 {
	var avgMs float64
	err := ho.db.QueryRowContext(ctx, `
		SELECT AVG(wr.execution_time_ms) 
		FROM workflow_results wr
		JOIN agent_assignments aa ON wr.workflow_id = aa.workflow_id
		WHERE aa.agent_id = ?
	`, agentID).Scan(&avgMs)

	if err != nil {
		return 0
	}
	return avgMs
}

// PlanHandoff determines the optimal target agent for a handoff
func (ho *HandoffOptimizer) PlanHandoff(ctx context.Context, workflowID, fromAgentID, targetAgentType string) (*HandoffPlan, error) {
	// Select best target
	target, err := ho.SelectBestAgent(ctx, targetAgentType)
	if err != nil {
		return nil, fmt.Errorf("no suitable agent for handoff: %w", err)
	}

	// Fetch workflow results so far
	var resultCount int
	ho.db.QueryRowContext(ctx, `
		SELECT COUNT(*) FROM workflow_results WHERE workflow_id = ?
	`, workflowID).Scan(&resultCount)

	plan := &HandoffPlan{
		WorkflowID:    workflowID,
		FromAgentID:   fromAgentID,
		ToAgentID:     target.AgentID,
		ToAgentType:   target.AgentType,
		Score:         target.Score,
		Reason:        target.Reason,
		ResultCount:   resultCount,
		PlannedAt:     time.Now(),
	}

	return plan, nil
}

// HandoffPlan describes a planned agent handoff
type HandoffPlan struct {
	WorkflowID  string    `json:"workflow_id"`
	FromAgentID string    `json:"from_agent_id"`
	ToAgentID   string    `json:"to_agent_id"`
	ToAgentType string    `json:"to_agent_type"`
	Score       float64   `json:"score"`
	Reason      string    `json:"reason"`
	ResultCount int       `json:"result_count"`
	PlannedAt   time.Time `json:"planned_at"`
}

// RebalanceAgents checks if any agents are overloaded and suggests moves
func (ho *HandoffOptimizer) RebalanceAgents(ctx context.Context) ([]RebalanceSuggestion, error) {
	rows, err := ho.db.QueryContext(ctx, `
		SELECT agent_id, agent_type, current_workload, max_workload
		FROM agent_configurations
		WHERE status = 'active'
		ORDER BY CAST(current_workload AS REAL) / max_workload DESC
	`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	type agentLoad struct {
		ID         string
		Type       string
		Current    int
		Max        int
		LoadFactor float64
	}

	var agents []agentLoad
	for rows.Next() {
		var a agentLoad
		if err := rows.Scan(&a.ID, &a.Type, &a.Current, &a.Max); err != nil {
			continue
		}
		if a.Max > 0 {
			a.LoadFactor = float64(a.Current) / float64(a.Max)
		}
		agents = append(agents, a)
	}

	var suggestions []RebalanceSuggestion

	for _, overloaded := range agents {
		if overloaded.LoadFactor < 0.8 {
			continue // Not overloaded
		}

		// Find a less loaded agent of the same type
		for _, candidate := range agents {
			if candidate.ID == overloaded.ID || candidate.Type != overloaded.Type {
				continue
			}
			if candidate.LoadFactor < 0.5 {
				suggestions = append(suggestions, RebalanceSuggestion{
					FromAgentID:     overloaded.ID,
					ToAgentID:       candidate.ID,
					AgentType:       overloaded.Type,
					FromLoadPercent: overloaded.LoadFactor * 100,
					ToLoadPercent:   candidate.LoadFactor * 100,
					Reason: fmt.Sprintf(
						"%s is at %.0f%% load, %s is at %.0f%% load",
						overloaded.ID, overloaded.LoadFactor*100,
						candidate.ID, candidate.LoadFactor*100,
					),
				})
				break
			}
		}
	}

	return suggestions, nil
}

// RebalanceSuggestion describes a suggested agent rebalance
type RebalanceSuggestion struct {
	FromAgentID     string  `json:"from_agent_id"`
	ToAgentID       string  `json:"to_agent_id"`
	AgentType       string  `json:"agent_type"`
	FromLoadPercent float64 `json:"from_load_percent"`
	ToLoadPercent   float64 `json:"to_load_percent"`
	Reason          string  `json:"reason"`
}