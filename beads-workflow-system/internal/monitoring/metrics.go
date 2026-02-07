package monitoring

import (
	"database/sql"
	"fmt"
	"sync"
	"time"
)

// MetricsCollector collects and aggregates performance metrics
type MetricsCollector struct {
	db      *sql.DB
	mu      sync.RWMutex
	gauges  map[string]float64
	counters map[string]int64
	timers  map[string][]time.Duration
}

// NewMetricsCollector creates a new metrics collector
func NewMetricsCollector(db *sql.DB) *MetricsCollector {
	return &MetricsCollector{
		db:       db,
		gauges:   make(map[string]float64),
		counters: make(map[string]int64),
		timers:   make(map[string][]time.Duration),
	}
}

// RecordWorkflowStart records a workflow start event
func (mc *MetricsCollector) RecordWorkflowStart(workflowID, workflowType, agentType string) {
	mc.mu.Lock()
	defer mc.mu.Unlock()

	mc.counters["workflows.started"]++
	mc.counters[fmt.Sprintf("workflows.started.%s", workflowType)]++
	mc.counters[fmt.Sprintf("agents.active.%s", agentType)]++
}

// RecordWorkflowEnd records a workflow completion
func (mc *MetricsCollector) RecordWorkflowEnd(workflowID, workflowType, agentType string, duration time.Duration, success bool) {
	mc.mu.Lock()
	defer mc.mu.Unlock()

	mc.timers[fmt.Sprintf("workflows.duration.%s", workflowType)] = append(
		mc.timers[fmt.Sprintf("workflows.duration.%s", workflowType)], duration,
	)
	mc.timers["workflows.duration.all"] = append(mc.timers["workflows.duration.all"], duration)

	if success {
		mc.counters["workflows.completed"]++
		mc.counters[fmt.Sprintf("workflows.completed.%s", workflowType)]++
	} else {
		mc.counters["workflows.failed"]++
		mc.counters[fmt.Sprintf("workflows.failed.%s", workflowType)]++
	}

	mc.counters[fmt.Sprintf("agents.active.%s", agentType)]--
	if mc.counters[fmt.Sprintf("agents.active.%s", agentType)] < 0 {
		mc.counters[fmt.Sprintf("agents.active.%s", agentType)] = 0
	}
}

// RecordStepExecution records a step execution
func (mc *MetricsCollector) RecordStepExecution(workflowType, stepName string, duration time.Duration, success bool) {
	mc.mu.Lock()
	defer mc.mu.Unlock()

	key := fmt.Sprintf("steps.duration.%s.%s", workflowType, stepName)
	mc.timers[key] = append(mc.timers[key], duration)

	if success {
		mc.counters[fmt.Sprintf("steps.success.%s.%s", workflowType, stepName)]++
	} else {
		mc.counters[fmt.Sprintf("steps.failure.%s.%s", workflowType, stepName)]++
	}
}

// SetGauge sets a gauge metric
func (mc *MetricsCollector) SetGauge(name string, value float64) {
	mc.mu.Lock()
	defer mc.mu.Unlock()
	mc.gauges[name] = value
}

// IncrementCounter increments a counter
func (mc *MetricsCollector) IncrementCounter(name string) {
	mc.mu.Lock()
	defer mc.mu.Unlock()
	mc.counters[name]++
}

// Snapshot returns a point-in-time snapshot of all metrics
type MetricsSnapshot struct {
	Timestamp time.Time              `json:"timestamp"`
	System    SystemMetrics          `json:"system"`
	Workflows WorkflowMetrics        `json:"workflows"`
	Agents    AgentMetrics           `json:"agents"`
	Steps     map[string]StepMetrics `json:"steps"`
}

// SystemMetrics holds system-level metrics
type SystemMetrics struct {
	Uptime       time.Duration `json:"uptime_seconds"`
	GoroutineCount int        `json:"goroutine_count"`
}

// WorkflowMetrics holds workflow-level aggregate metrics
type WorkflowMetrics struct {
	TotalStarted   int64              `json:"total_started"`
	TotalCompleted int64              `json:"total_completed"`
	TotalFailed    int64              `json:"total_failed"`
	ActiveCount    int64              `json:"active_count"`
	SuccessRate    float64            `json:"success_rate"`
	AvgDurationMs  float64            `json:"avg_duration_ms"`
	P95DurationMs  float64            `json:"p95_duration_ms"`
	P99DurationMs  float64            `json:"p99_duration_ms"`
	ByType         map[string]TypeMetrics `json:"by_type"`
}

// TypeMetrics holds per-type workflow metrics
type TypeMetrics struct {
	Started       int64   `json:"started"`
	Completed     int64   `json:"completed"`
	Failed        int64   `json:"failed"`
	SuccessRate   float64 `json:"success_rate"`
	AvgDurationMs float64 `json:"avg_duration_ms"`
}

// AgentMetrics holds agent-level metrics
type AgentMetrics struct {
	TotalRegistered int           `json:"total_registered"`
	TotalActive     int           `json:"total_active"`
	ByType          map[string]AgentTypeMetrics `json:"by_type"`
}

// AgentTypeMetrics holds per-agent-type metrics
type AgentTypeMetrics struct {
	Active           int     `json:"active"`
	TotalAssignments int64   `json:"total_assignments"`
	AvgWorkload      float64 `json:"avg_workload"`
}

// StepMetrics holds per-step metrics
type StepMetrics struct {
	TotalExecutions int64   `json:"total_executions"`
	Successes       int64   `json:"successes"`
	Failures        int64   `json:"failures"`
	SuccessRate     float64 `json:"success_rate"`
	AvgDurationMs   float64 `json:"avg_duration_ms"`
}

// GetSnapshot returns a point-in-time snapshot of all metrics
func (mc *MetricsCollector) GetSnapshot() *MetricsSnapshot {
	mc.mu.RLock()
	defer mc.mu.RUnlock()

	started := mc.counters["workflows.started"]
	completed := mc.counters["workflows.completed"]
	failed := mc.counters["workflows.failed"]

	successRate := 0.0
	if started > 0 {
		successRate = float64(completed) / float64(started) * 100
	}

	snapshot := &MetricsSnapshot{
		Timestamp: time.Now(),
		Workflows: WorkflowMetrics{
			TotalStarted:   started,
			TotalCompleted: completed,
			TotalFailed:    failed,
			ActiveCount:    started - completed - failed,
			SuccessRate:    successRate,
			AvgDurationMs:  mc.avgDuration("workflows.duration.all"),
			P95DurationMs:  mc.percentileDuration("workflows.duration.all", 0.95),
			P99DurationMs:  mc.percentileDuration("workflows.duration.all", 0.99),
			ByType:         mc.getTypeMetrics(),
		},
		Steps: mc.getStepMetrics(),
	}

	// Pull agent metrics from DB
	snapshot.Agents = mc.getAgentMetricsFromDB()

	return snapshot
}

// getTypeMetrics computes per-type metrics
func (mc *MetricsCollector) getTypeMetrics() map[string]TypeMetrics {
	types := map[string]TypeMetrics{}
	workflowTypes := []string{"research", "poc", "documentation", "validation"}

	for _, wt := range workflowTypes {
		started := mc.counters[fmt.Sprintf("workflows.started.%s", wt)]
		completed := mc.counters[fmt.Sprintf("workflows.completed.%s", wt)]
		failed := mc.counters[fmt.Sprintf("workflows.failed.%s", wt)]

		if started == 0 {
			continue
		}

		types[wt] = TypeMetrics{
			Started:       started,
			Completed:     completed,
			Failed:        failed,
			SuccessRate:   float64(completed) / float64(started) * 100,
			AvgDurationMs: mc.avgDuration(fmt.Sprintf("workflows.duration.%s", wt)),
		}
	}

	return types
}

// getStepMetrics computes per-step metrics
func (mc *MetricsCollector) getStepMetrics() map[string]StepMetrics {
	steps := make(map[string]StepMetrics)

	// Scan counters for step metrics
	for key, value := range mc.counters {
		var workflowType, stepName, status string
		n, _ := fmt.Sscanf(key, "steps.%s.%s.%s", &status, &workflowType, &stepName)
		if n < 3 {
			continue
		}

		fullKey := fmt.Sprintf("%s/%s", workflowType, stepName)
		sm := steps[fullKey]

		if status == "success" {
			sm.Successes = value
		} else if status == "failure" {
			sm.Failures = value
		}

		sm.TotalExecutions = sm.Successes + sm.Failures
		if sm.TotalExecutions > 0 {
			sm.SuccessRate = float64(sm.Successes) / float64(sm.TotalExecutions) * 100
		}

		durKey := fmt.Sprintf("steps.duration.%s.%s", workflowType, stepName)
		sm.AvgDurationMs = mc.avgDuration(durKey)

		steps[fullKey] = sm
	}

	return steps
}

// getAgentMetricsFromDB pulls agent metrics from coordination DB
func (mc *MetricsCollector) getAgentMetricsFromDB() AgentMetrics {
	metrics := AgentMetrics{
		ByType: make(map[string]AgentTypeMetrics),
	}

	if mc.db == nil {
		return metrics
	}

	// Total agents
	mc.db.QueryRow("SELECT COUNT(*) FROM agent_configurations").Scan(&metrics.TotalRegistered)
	mc.db.QueryRow("SELECT COUNT(*) FROM agent_configurations WHERE status = 'active'").Scan(&metrics.TotalActive)

	// Per-type metrics
	rows, err := mc.db.Query(`
		SELECT agent_type, COUNT(*) as cnt, 
			   AVG(CAST(current_workload AS REAL) / max_workload) as avg_load
		FROM agent_configurations 
		WHERE status = 'active'
		GROUP BY agent_type
	`)
	if err == nil {
		defer rows.Close()
		for rows.Next() {
			var agentType string
			var cnt int
			var avgLoad float64
			if err := rows.Scan(&agentType, &cnt, &avgLoad); err == nil {
				metrics.ByType[agentType] = AgentTypeMetrics{
					Active:      cnt,
					AvgWorkload: avgLoad * 100,
				}
			}
		}
	}

	return metrics
}

// avgDuration calculates the average duration for a timer key
func (mc *MetricsCollector) avgDuration(key string) float64 {
	durations := mc.timers[key]
	if len(durations) == 0 {
		return 0
	}

	var total time.Duration
	for _, d := range durations {
		total += d
	}

	return float64(total.Milliseconds()) / float64(len(durations))
}

// percentileDuration calculates a percentile duration
func (mc *MetricsCollector) percentileDuration(key string, percentile float64) float64 {
	durations := mc.timers[key]
	if len(durations) == 0 {
		return 0
	}

	// Simple sort-based percentile calculation
	sorted := make([]time.Duration, len(durations))
	copy(sorted, durations)
	sortDurations(sorted)

	idx := int(float64(len(sorted)-1) * percentile)
	return float64(sorted[idx].Milliseconds())
}

// sortDurations sorts a slice of durations (insertion sort for small slices)
func sortDurations(d []time.Duration) {
	for i := 1; i < len(d); i++ {
		key := d[i]
		j := i - 1
		for j >= 0 && d[j] > key {
			d[j+1] = d[j]
			j--
		}
		d[j+1] = key
	}
}

// FlushToDB persists current metrics to the analytics table
func (mc *MetricsCollector) FlushToDB() error {
	mc.mu.RLock()
	defer mc.mu.RUnlock()

	if mc.db == nil {
		return nil
	}

	workflowTypes := []string{"research", "poc", "documentation", "validation"}
	today := time.Now().Format("2006-01-02")

	for _, wt := range workflowTypes {
		started := mc.counters[fmt.Sprintf("workflows.started.%s", wt)]
		completed := mc.counters[fmt.Sprintf("workflows.completed.%s", wt)]
		failed := mc.counters[fmt.Sprintf("workflows.failed.%s", wt)]

		if started == 0 {
			continue
		}

		avgDuration := mc.avgDuration(fmt.Sprintf("workflows.duration.%s", wt))

		_, err := mc.db.Exec(`
			INSERT INTO workflow_analytics 
			(date, workflow_type, agent_type, total_workflows, successful_workflows, failed_workflows, avg_execution_time_ms)
			VALUES (?, ?, ?, ?, ?, ?, ?)
			ON CONFLICT(date, workflow_type, agent_type) DO UPDATE SET
				total_workflows = excluded.total_workflows,
				successful_workflows = excluded.successful_workflows,
				failed_workflows = excluded.failed_workflows,
				avg_execution_time_ms = excluded.avg_execution_time_ms
		`, today, wt, wt, started, completed, failed, int(avgDuration))

		if err != nil {
			return fmt.Errorf("failed to flush metrics for %s: %w", wt, err)
		}
	}

	return nil
}

// Reset clears all in-memory metrics
func (mc *MetricsCollector) Reset() {
	mc.mu.Lock()
	defer mc.mu.Unlock()

	mc.gauges = make(map[string]float64)
	mc.counters = make(map[string]int64)
	mc.timers = make(map[string][]time.Duration)
}