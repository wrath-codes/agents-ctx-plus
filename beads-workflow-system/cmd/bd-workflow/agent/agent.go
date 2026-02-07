package agent

import (
	"database/sql"
	"fmt"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// NewAgentCommand creates the agent command
func NewAgentCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "agent",
		Short: "Manage agents",
		Long:  "Register, monitor, and manage workflow agents.",
	}

	cmd.AddCommand(newRegisterCommand())
	cmd.AddCommand(newAgentStatusCommand())
	cmd.AddCommand(newAgentListCommand())

	return cmd
}

func newRegisterCommand() *cobra.Command {
	var (
		agentType string
		maxWorkload int
	)

	cmd := &cobra.Command{
		Use:   "register [agent-id]",
		Short: "Register a new agent",
		Long:  "Registers a new agent with the workflow system.",
		Args:  cobra.ExactArgs(1),
		Example: `  bd-workflow agent register research-agent-01 --type research
  bd-workflow agent register poc-agent-01 --type poc --max-workload 3`,
		RunE: func(cmd *cobra.Command, args []string) error {
			agentID := args[0]

			dbPath := viper.GetString("db-path")
			db, err := sql.Open("sqlite3", dbPath+"/coordination.db")
			if err != nil {
				return err
			}
			defer db.Close()

			configJSON := `{"timeout": "30m", "retry_policy": "exponential_backoff"}`
			capsJSON := `{"capabilities": ["workflow_execution"]}`
			perfJSON := `{}`

			_, err = db.Exec(`
				INSERT INTO agent_configurations 
				(id, agent_type, agent_id, configuration, capabilities, max_workload, current_workload, status, last_heartbeat, performance_metrics, created_at, updated_at)
				VALUES (?, ?, ?, ?, ?, ?, 0, 'active', ?, ?, ?, ?)
				ON CONFLICT(agent_type, agent_id) DO UPDATE SET
				status = 'active',
				updated_at = excluded.updated_at
			`,
				generateID(),
				agentType,
				agentID,
				configJSON,
				capsJSON,
				maxWorkload,
				time.Now(),
				perfJSON,
				time.Now(),
				time.Now(),
			)

			if err != nil {
				return fmt.Errorf("failed to register agent: %w", err)
			}

			fmt.Printf("✓ Agent %s registered successfully\n", agentID)
			fmt.Printf("  Type:        %s\n", agentType)
			fmt.Printf("  Max Workload: %d\n", maxWorkload)

			return nil
		},
	}

	cmd.Flags().StringVar(&agentType, "type", "research", "Agent type (research, poc, documentation, validation)")
	cmd.Flags().IntVar(&maxWorkload, "max-workload", 5, "Maximum concurrent workflows")

	return cmd
}

func newAgentStatusCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "status [agent-id]",
		Short: "Get agent status",
		Long:  "Displays detailed status information for a specific agent.",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			agentID := args[0]

			dbPath := viper.GetString("db-path")
			db, err := sql.Open("sqlite3", dbPath+"/coordination.db")
			if err != nil {
				return err
			}
			defer db.Close()

			var agentType, status string
			var maxWorkload, currentWorkload int
			var lastHeartbeat time.Time

			err = db.QueryRow(`
				SELECT agent_type, status, max_workload, current_workload, last_heartbeat
				FROM agent_configurations WHERE agent_id = ?
			`, agentID).Scan(&agentType, &status, &maxWorkload, &currentWorkload, &lastHeartbeat)

			if err == sql.ErrNoRows {
				return fmt.Errorf("agent not found: %s", agentID)
			}
			if err != nil {
				return err
			}

			fmt.Printf("Agent: %s\n", agentID)
			fmt.Printf("============\n")
			fmt.Printf("Type:            %s\n", agentType)
			fmt.Printf("Status:          %s\n", status)
			fmt.Printf("Current Load:    %d/%d\n", currentWorkload, maxWorkload)
			fmt.Printf("Last Heartbeat:  %s\n", lastHeartbeat.Format(time.RFC3339))

			// Get active assignments
			rows, err := db.Query(`
				SELECT workflow_id, step_name, assigned_at
				FROM agent_assignments 
				WHERE agent_id = ? AND status IN ('assigned', 'started')
			`, agentID)
			if err != nil {
				return err
			}
			defer rows.Close()

			fmt.Printf("\nActive Assignments:\n")
			count := 0
			for rows.Next() {
				var workflowID, stepName string
				var assignedAt time.Time
				if err := rows.Scan(&workflowID, &stepName, &assignedAt); err != nil {
					continue
				}
				if stepName == "" {
					stepName = "N/A"
				}
				fmt.Printf("  - %s [%s] since %s\n", workflowID, stepName, assignedAt.Format("15:04"))
				count++
			}

			if count == 0 {
				fmt.Printf("  No active assignments\n")
			}

			return nil
		},
	}
}

func newAgentListCommand() *cobra.Command {
	var agentType string

	cmd := &cobra.Command{
		Use:   "list",
		Short: "List agents",
		Long:  "Lists all registered agents with their status and workload.",
		RunE: func(cmd *cobra.Command, args []string) error {
			dbPath := viper.GetString("db-path")
			db, err := sql.Open("sqlite3", dbPath+"/coordination.db")
			if err != nil {
				return err
			}
			defer db.Close()

			query := `
				SELECT agent_id, agent_type, status, max_workload, current_workload, last_heartbeat
				FROM agent_configurations WHERE 1=1
			`
			var params []interface{}

			if agentType != "" {
				query += " AND agent_type = ?"
				params = append(params, agentType)
			}

			query += " ORDER BY agent_type, agent_id"

			rows, err := db.Query(query, params...)
			if err != nil {
				return err
			}
			defer rows.Close()

			fmt.Printf("%-25s %-12s %-10s %-12s %s\n", "AGENT ID", "TYPE", "STATUS", "WORKLOAD", "LAST HEARTBEAT")
			fmt.Println(strings.Repeat("-", 90))

			count := 0
			for rows.Next() {
				var agentID, agentType, status string
				var maxWorkload, currentWorkload int
				var lastHeartbeat time.Time

				if err := rows.Scan(&agentID, &agentType, &status, &maxWorkload, &currentWorkload, &lastHeartbeat); err != nil {
					continue
				}

				workloadStr := fmt.Sprintf("%d/%d", currentWorkload, maxWorkload)
				heartbeatStr := lastHeartbeat.Format("2006-01-02 15:04")

				fmt.Printf("%-25s %-12s %-10s %-12s %s\n",
					agentID, agentType, status, workloadStr, heartbeatStr)
				count++
			}

			fmt.Printf("\nTotal: %d agents\n", count)
			return nil
		},
	}

	cmd.Flags().StringVar(&agentType, "type", "", "Filter by agent type")

	return cmd
}

func generateID() string {
	return fmt.Sprintf("agent-%d", time.Now().UnixNano())
}