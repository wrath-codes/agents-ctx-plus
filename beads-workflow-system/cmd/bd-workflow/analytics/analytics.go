package analytics

import (
	"database/sql"
	"fmt"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

func NewAnalyticsCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "analytics",
		Short: "View analytics and metrics",
		Long:  "View performance analytics, workflow statistics, and system metrics.",
	}

	cmd.AddCommand(newPerformanceCommand())
	cmd.AddCommand(newSummaryCommand())

	return cmd
}

func newPerformanceCommand() *cobra.Command {
	var period string
	var agentType string
	var format string

	cmd := &cobra.Command{
		Use:   "performance",
		Short: "View performance analytics",
		Long:  "Displays performance metrics for workflows and agents.",
		RunE: func(cmd *cobra.Command, args []string) error {
			dbPath := viper.GetString("db-path")
			db, err := sql.Open("sqlite3", dbPath+"/coordination.db")
			if err != nil {
				return err
			}
			defer db.Close()

			duration, err := parsePeriod(period)
			if err != nil {
				return err
			}
			since := time.Now().Add(-duration)

			query := buildQuery(agentType)
			params := []interface{}{since}
			if agentType != "" {
				params = append(params, agentType)
			}

			var total, completed, failed int
			err = db.QueryRow(query, params...).Scan(&total, &completed, &failed)
			if err != nil {
				return err
			}

			var avgExecutionTime float64
			err = db.QueryRow("SELECT AVG(execution_time_ms) FROM workflow_results WHERE created_at >= ?", since).Scan(&avgExecutionTime)
			if err != nil {
				avgExecutionTime = 0
			}

			successRate := 0.0
			if total > 0 {
				successRate = float64(completed) / float64(total) * 100
			}

			if format == "json" {
				fmt.Printf("{\"period\": \"%s\", \"workflows\": {\"total\": %d, \"completed\": %d, \"failed\": %d, \"success_rate\": %.1f}, \"performance\": {\"avg_execution_time_ms\": %.0f}}\n",
					period, total, completed, failed, successRate, avgExecutionTime)
			} else {
				fmt.Printf("Performance Analytics (%s)\n", period)
				fmt.Println(strings.Repeat("=", 50))
				fmt.Printf("Workflows:\n")
				fmt.Printf("  Total:       %d\n", total)
				fmt.Printf("  Completed:   %d\n", completed)
				fmt.Printf("  Failed:      %d\n", failed)
				fmt.Printf("  Success Rate: %.1f%%\n", successRate)
				fmt.Printf("\nPerformance:\n")
				fmt.Printf("  Avg Execution Time: %.0f ms\n", avgExecutionTime)
			}

			return nil
		},
	}

	cmd.Flags().StringVar(&period, "period", "7d", "Time period (1d, 7d, 30d)")
	cmd.Flags().StringVar(&agentType, "agent-type", "", "Filter by agent type")
	cmd.Flags().StringVar(&format, "format", "table", "Output format (table, json)")

	return cmd
}

func buildQuery(agentType string) string {
	query := "SELECT COUNT(*) as total, SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed, SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed FROM workflow_mappings WHERE created_at >= ?"
	if agentType != "" {
		query += " AND workflow_type = ?"
	}
	return query
}

func newSummaryCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "summary",
		Short: "View system summary",
		Long:  "Displays a summary of the workflow system status.",
		RunE: func(cmd *cobra.Command, args []string) error {
			dbPath := viper.GetString("db-path")
			db, err := sql.Open("sqlite3", dbPath+"/coordination.db")
			if err != nil {
				return err
			}
			defer db.Close()

			rows, err := db.Query("SELECT status, COUNT(*) FROM workflow_mappings GROUP BY status")
			if err != nil {
				return err
			}
			defer rows.Close()

			workflowCounts := make(map[string]int)
			for rows.Next() {
				var status string
				var count int
				if err := rows.Scan(&status, &count); err != nil {
					continue
				}
				workflowCounts[status] = count
			}

			var totalAgents int
			db.QueryRow("SELECT COUNT(*) FROM agent_configurations").Scan(&totalAgents)

			var activeAgents int
			db.QueryRow("SELECT COUNT(*) FROM agent_configurations WHERE status = 'active'").Scan(&activeAgents)

			var recentWorkflows int
			db.QueryRow("SELECT COUNT(*) FROM workflow_mappings WHERE created_at >= datetime('now', '-1 day')").Scan(&recentWorkflows)

			fmt.Println("System Summary")
			fmt.Println(strings.Repeat("=", 50))
			fmt.Printf("\nWorkflows:\n")
			fmt.Printf("  Active:     %d\n", workflowCounts["active"])
			fmt.Printf("  Completed:  %d\n", workflowCounts["completed"])
			fmt.Printf("  Failed:     %d\n", workflowCounts["failed"])
			fmt.Printf("  Paused:     %d\n", workflowCounts["paused"])
			fmt.Printf("  Total:      %d\n", sumMap(workflowCounts))
			fmt.Printf("\nAgents:\n")
			fmt.Printf("  Total:      %d\n", totalAgents)
			fmt.Printf("  Active:     %d\n", activeAgents)
			fmt.Printf("\nRecent Activity (24h):\n")
			fmt.Printf("  New workflows: %d\n", recentWorkflows)

			return nil
		},
	}
}

func parsePeriod(period string) (time.Duration, error) {
	switch period {
	case "1d":
		return 24 * time.Hour, nil
	case "7d":
		return 7 * 24 * time.Hour, nil
	case "30d":
		return 30 * 24 * time.Hour, nil
	default:
		return 7 * 24 * time.Hour, fmt.Errorf("invalid period: %s (use 1d, 7d, or 30d)", period)
	}
}

func sumMap(m map[string]int) int {
	sum := 0
	for _, v := range m {
		sum += v
	}
	return sum
}