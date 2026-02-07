package workflow

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
	"github.com/your-org/beads-workflow-system/internal/agents"
	"github.com/your-org/beads-workflow-system/internal/beads"
	"github.com/your-org/beads-workflow-system/internal/bridge"
	"github.com/your-org/beads-workflow-system/internal/database"
	"github.com/your-org/beads-workflow-system/internal/monitoring"
	"github.com/your-org/beads-workflow-system/internal/tempolite"
	"github.com/your-org/beads-workflow-system/pkg/models"
)

// NewWorkflowCommand creates the workflow command
func NewWorkflowCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "workflow",
		Short: "Manage workflows",
		Long:  "Create, monitor, and manage workflow executions.",
	}

	cmd.AddCommand(newStartCommand())
	cmd.AddCommand(newStatusCommand())
	cmd.AddCommand(newListCommand())
	cmd.AddCommand(newCancelCommand())
	cmd.AddCommand(newResultsCommand())
	cmd.AddCommand(newExecuteCommand())
	cmd.AddCommand(newTemplateCommand())

	return cmd
}

func newStartCommand() *cobra.Command {
	var (
		agentType  string
		priority   int
		variables  []string
		templateID string
	)

	cmd := &cobra.Command{
		Use:   "start [workflow-type] [title]",
		Short: "Start a new workflow",
		Long:  "Starts a new workflow of the specified type with the given title.",
		Args:  cobra.ExactArgs(2),
		Example: `  bd-workflow workflow start research "Analyze Rust async frameworks" --agent research
  bd-workflow workflow start poc "Implement authentication" --agent poc --priority 1
  bd-workflow workflow start documentation "API docs" --template comprehensive`,
		RunE: func(cmd *cobra.Command, args []string) error {
			workflowType := args[0]
			title := args[1]

			validTypes := []string{"research", "poc", "documentation", "validation"}
			if !contains(validTypes, workflowType) {
				return fmt.Errorf("invalid workflow type: %s (must be one of: %s)",
					workflowType, strings.Join(validTypes, ", "))
			}

			vars := make(map[string]interface{})
			for _, v := range variables {
				parts := strings.SplitN(v, "=", 2)
				if len(parts) == 2 {
					vars[parts[0]] = parts[1]
				}
			}

			coordBridge, err := initializeBridge()
			if err != nil {
				return err
			}

			req := &models.StartWorkflowRequest{
				IssueTitle:   title,
				WorkflowType: workflowType,
				AgentType:    agentType,
				Priority:     priority,
				Variables:    vars,
				TemplateID:   templateID,
			}

			ctx := context.Background()
			workflow, err := coordBridge.StartWorkflow(ctx, req)
			if err != nil {
				return fmt.Errorf("failed to start workflow: %w", err)
			}

			fmt.Printf("Workflow started successfully\n")
			fmt.Printf("  Workflow ID:    %s\n", workflow.ID)
			fmt.Printf("  Beads Issue ID: %s\n", workflow.BeadsIssueID)
			fmt.Printf("  Type:           %s\n", workflow.Type)
			fmt.Printf("  Status:         %s\n", workflow.Status)
			fmt.Printf("  Agent:          %s\n", workflow.AgentID)
			fmt.Printf("  Started:        %s\n", workflow.StartedAt.Format(time.RFC3339))

			return nil
		},
	}

	cmd.Flags().StringVar(&agentType, "agent", "", "Agent type to assign (research, poc, documentation, validation)")
	cmd.Flags().IntVar(&priority, "priority", 2, "Workflow priority (0-3, where 0 is highest)")
	cmd.Flags().StringArrayVar(&variables, "variable", []string{}, "Workflow variables (key=value)")
	cmd.Flags().StringVar(&templateID, "template", "", "Template ID to use")

	return cmd
}

func newExecuteCommand() *cobra.Command {
	var (
		agentType  string
		priority   int
		variables  []string
		templateID string
	)

	cmd := &cobra.Command{
		Use:   "execute [workflow-type] [title]",
		Short: "Start and execute a workflow immediately",
		Long:  "Creates a workflow and runs the agent pipeline to completion.",
		Args:  cobra.ExactArgs(2),
		Example: `  bd-workflow workflow execute research "Analyze Rust async frameworks" --agent research
  bd-workflow workflow execute poc "Build auth module" --agent poc --variable language=rust
  bd-workflow workflow execute documentation "Generate API docs" --template docs-comprehensive`,
		RunE: func(cmd *cobra.Command, args []string) error {
			workflowType := args[0]
			title := args[1]

			validTypes := []string{"research", "poc", "documentation", "validation"}
			if !contains(validTypes, workflowType) {
				return fmt.Errorf("invalid workflow type: %s (must be one of: %s)",
					workflowType, strings.Join(validTypes, ", "))
			}

			if agentType == "" {
				agentType = workflowType
			}

			vars := make(map[string]interface{})
			for _, v := range variables {
				parts := strings.SplitN(v, "=", 2)
				if len(parts) == 2 {
					vars[parts[0]] = parts[1]
				}
			}

			// If template specified, merge its variables
			if templateID != "" {
				tm := agents.NewTemplateManager()
				tmpl, err := tm.GetTemplate(templateID)
				if err != nil {
					return fmt.Errorf("template not found: %w", err)
				}
				for k, v := range tmpl.Variables {
					if _, exists := vars[k]; !exists {
						vars[k] = v
					}
				}
			}

			// Initialize bridge
			coordBridge, err := initializeBridge()
			if err != nil {
				return err
			}

			// Create workflow
			req := &models.StartWorkflowRequest{
				IssueTitle:   title,
				WorkflowType: workflowType,
				AgentType:    agentType,
				Priority:     priority,
				Variables:    vars,
				TemplateID:   templateID,
			}

			ctx := context.Background()
			workflow, err := coordBridge.StartWorkflow(ctx, req)
			if err != nil {
				return fmt.Errorf("failed to start workflow: %w", err)
			}

			fmt.Printf("Workflow %s started, executing...\n\n", workflow.ID)

			// Get agent from registry
			agent, err := agents.DefaultRegistry.Get(workflowType, workflow.AgentID)
			if err != nil {
				return fmt.Errorf("failed to get agent: %w", err)
			}

			// Print steps
			steps := agent.GetSteps()
			fmt.Printf("Agent: %s (%s)\n", agent.GetID(), agent.GetType())
			fmt.Printf("Steps: %d\n\n", len(steps))

			// Execute
			startTime := time.Now()

			// Set up metrics collector
			db, _ := openCoordinationDB()
			if db != nil {
				defer db.Close()
			}
			metrics := monitoring.NewMetricsCollector(db)
			metrics.RecordWorkflowStart(workflow.ID, workflowType, agentType)

			for i, step := range steps {
				fmt.Printf("  [%d/%d] %s ... ", i+1, len(steps), step.Name)
				stepStart := time.Now()

				_, stepErr := agents.ExecuteStep(ctx, step, map[string]interface{}{
					"workflow_id": workflow.ID,
					"step_number": i + 1,
					"total_steps": len(steps),
					"variables":   vars,
					"results":     make(map[string]interface{}),
				})

				stepDur := time.Since(stepStart)
				metrics.RecordStepExecution(workflowType, step.Name, stepDur, stepErr == nil)

				if stepErr != nil {
					fmt.Printf("FAILED (%s)\n", stepDur.Round(time.Millisecond))
					fmt.Printf("         Error: %v\n", stepErr)
				} else {
					fmt.Printf("OK (%s)\n", stepDur.Round(time.Millisecond))
				}
			}

			// Execute agent to get final result
			result, err := agent.Execute(ctx, workflow)
			totalDuration := time.Since(startTime)
			metrics.RecordWorkflowEnd(workflow.ID, workflowType, agentType, totalDuration, err == nil)

			fmt.Println()

			if err != nil {
				coordBridge.UpdateWorkflowStatus(ctx, workflow.ID, models.WorkflowStatusFailed)
				fmt.Printf("Workflow FAILED after %s\n", totalDuration.Round(time.Millisecond))
				fmt.Printf("Error: %v\n", err)
				return nil
			}

			// Store results
			coordBridge.StoreResults(ctx, workflow.ID, result)
			coordBridge.UpdateWorkflowStatus(ctx, workflow.ID, models.WorkflowStatusCompleted)

			// Flush metrics
			metrics.FlushToDB()

			fmt.Printf("Workflow completed in %s\n", totalDuration.Round(time.Millisecond))
			fmt.Printf("  Confidence: %.2f\n", result.ConfidenceScore)
			fmt.Printf("  Quality:    %.1f/10\n", result.QualityScore)
			fmt.Printf("  Artifacts:  %s\n", strings.Join(result.Artifacts, ", "))

			return nil
		},
	}

	cmd.Flags().StringVar(&agentType, "agent", "", "Agent type (defaults to workflow type)")
	cmd.Flags().IntVar(&priority, "priority", 2, "Workflow priority (0-3)")
	cmd.Flags().StringArrayVar(&variables, "variable", []string{}, "Workflow variables (key=value)")
	cmd.Flags().StringVar(&templateID, "template", "", "Template ID to use")

	return cmd
}

func newTemplateCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "template",
		Short: "Manage workflow templates",
	}

	cmd.AddCommand(newTemplateListCommand())
	cmd.AddCommand(newTemplateShowCommand())

	return cmd
}

func newTemplateListCommand() *cobra.Command {
	var agentType string

	cmd := &cobra.Command{
		Use:   "list",
		Short: "List available workflow templates",
		RunE: func(cmd *cobra.Command, args []string) error {
			tm := agents.NewTemplateManager()
			templates := tm.ListTemplates(agentType)

			fmt.Printf("%-22s %-15s %-14s %s\n", "ID", "AGENT TYPE", "STEPS", "DESCRIPTION")
			fmt.Println(strings.Repeat("-", 80))

			for _, t := range templates {
				fmt.Printf("%-22s %-15s %-14d %s\n", t.ID, t.AgentType, len(t.Steps), t.Description)
			}

			fmt.Printf("\nTotal: %d templates\n", len(templates))
			return nil
		},
	}

	cmd.Flags().StringVar(&agentType, "type", "", "Filter by agent type")

	return cmd
}

func newTemplateShowCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "show [template-id]",
		Short: "Show template details",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			tm := agents.NewTemplateManager()
			tmpl, err := tm.GetTemplate(args[0])
			if err != nil {
				return err
			}

			fmt.Printf("Template: %s\n", tmpl.ID)
			fmt.Printf("================\n")
			fmt.Printf("Name:        %s\n", tmpl.Name)
			fmt.Printf("Description: %s\n", tmpl.Description)
			fmt.Printf("Agent Type:  %s\n", tmpl.AgentType)
			fmt.Printf("Timeout:     %s\n", tmpl.Config.Timeout)
			fmt.Printf("Max Retries: %d\n", tmpl.Config.MaxRetries)
			fmt.Printf("\nSteps:\n")
			for i, step := range tmpl.Steps {
				fmt.Printf("  %d. %-30s timeout=%s retries=%d\n", i+1, step.Name, step.Timeout, step.RetryCount)
				fmt.Printf("     %s\n", step.Description)
			}

			if len(tmpl.Variables) > 0 {
				fmt.Printf("\nDefault Variables:\n")
				for k, v := range tmpl.Variables {
					fmt.Printf("  %s = %v\n", k, v)
				}
			}

			return nil
		},
	}
}

func newStatusCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "status [workflow-id]",
		Short: "Get workflow status",
		Long:  "Displays detailed status information for a specific workflow.",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			workflowID := args[0]

			coordBridge, err := initializeBridge()
			if err != nil {
				return err
			}

			ctx := context.Background()
			workflow, err := coordBridge.GetWorkflow(ctx, workflowID)
			if err != nil {
				return err
			}

			fmt.Printf("Workflow: %s\n", workflow.ID)
			fmt.Printf("================\n")
			fmt.Printf("Beads Issue:    %s\n", workflow.BeadsIssueID)
			fmt.Printf("Type:           %s\n", workflow.Type)
			fmt.Printf("Status:         %s\n", workflow.Status)
			fmt.Printf("Priority:       %d\n", workflow.Priority)
			fmt.Printf("Agent:          %s\n", workflow.AgentID)
			fmt.Printf("Started:        %s\n", workflow.StartedAt.Format(time.RFC3339))
			if workflow.CompletedAt != nil {
				fmt.Printf("Completed:      %s\n", workflow.CompletedAt.Format(time.RFC3339))
			}

			// Show results count
			results, _ := coordBridge.GetResults(ctx, workflowID)
			if len(results) > 0 {
				fmt.Printf("\nResults: %d\n", len(results))
				for _, r := range results {
					fmt.Printf("  - %s: confidence=%.2f quality=%.1f duration=%dms\n",
						r.ResultType, r.ConfidenceScore, r.QualityScore, r.ExecutionTimeMs)
				}
			}

			return nil
		},
	}
}

func newListCommand() *cobra.Command {
	var (
		status    string
		agentType string
		limit     int
	)

	cmd := &cobra.Command{
		Use:   "list",
		Short: "List workflows",
		Long:  "Lists workflows with optional filtering.",
		RunE: func(cmd *cobra.Command, args []string) error {
			db, err := openCoordinationDB()
			if err != nil {
				return err
			}
			defer db.Close()

			query := "SELECT tempolite_workflow_id, beads_issue_id, workflow_type, status, priority, created_at FROM workflow_mappings WHERE 1=1"
			var params []interface{}

			if status != "" {
				query += " AND status = ?"
				params = append(params, status)
			}

			if agentType != "" {
				query += " AND workflow_type = ?"
				params = append(params, agentType)
			}

			query += " ORDER BY created_at DESC LIMIT ?"
			params = append(params, limit)

			rows, err := db.Query(query, params...)
			if err != nil {
				return err
			}
			defer rows.Close()

			fmt.Printf("%-25s %-14s %-12s %-10s %s\n", "WORKFLOW ID", "TYPE", "STATUS", "PRIORITY", "CREATED")
			fmt.Println(strings.Repeat("-", 85))

			count := 0
			for rows.Next() {
				var workflowID, beadsID, workflowType, wfStatus string
				var priority int
				var createdAt time.Time

				if err := rows.Scan(&workflowID, &beadsID, &workflowType, &wfStatus, &priority, &createdAt); err != nil {
					continue
				}

				fmt.Printf("%-25s %-14s %-12s %-10d %s\n",
					workflowID, workflowType, wfStatus, priority, createdAt.Format("2006-01-02 15:04"))
				count++
			}

			fmt.Printf("\nTotal: %d workflows\n", count)
			return nil
		},
	}

	cmd.Flags().StringVar(&status, "status", "", "Filter by status (active, completed, failed, paused)")
	cmd.Flags().StringVar(&agentType, "agent-type", "", "Filter by workflow type")
	cmd.Flags().IntVar(&limit, "limit", 20, "Maximum number of workflows to show")

	return cmd
}

func newCancelCommand() *cobra.Command {
	var reason string

	cmd := &cobra.Command{
		Use:   "cancel [workflow-id]",
		Short: "Cancel a workflow",
		Long:  "Cancels a running or pending workflow.",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			workflowID := args[0]

			if reason == "" {
				reason = "User request"
			}

			coordBridge, err := initializeBridge()
			if err != nil {
				return err
			}

			ctx := context.Background()
			if err := coordBridge.CancelWorkflow(ctx, workflowID, reason); err != nil {
				return err
			}

			fmt.Printf("Workflow %s cancelled\n", workflowID)
			return nil
		},
	}

	cmd.Flags().StringVar(&reason, "reason", "", "Reason for cancellation")

	return cmd
}

func newResultsCommand() *cobra.Command {
	var format string

	cmd := &cobra.Command{
		Use:   "results [workflow-id]",
		Short: "Get workflow results",
		Long:  "Retrieves and displays results from a completed workflow.",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			workflowID := args[0]

			coordBridge, err := initializeBridge()
			if err != nil {
				return err
			}

			ctx := context.Background()
			results, err := coordBridge.GetResults(ctx, workflowID)
			if err != nil {
				return err
			}

			if len(results) == 0 {
				fmt.Println("No results found for this workflow")
				return nil
			}

			if format == "json" {
				data, _ := json.MarshalIndent(results, "", "  ")
				fmt.Println(string(data))
				return nil
			}

			fmt.Printf("Results for workflow %s:\n", workflowID)
			fmt.Println(strings.Repeat("=", 60))

			for i, result := range results {
				fmt.Printf("\nResult #%d:\n", i+1)
				fmt.Printf("  Type:           %s\n", result.ResultType)
				fmt.Printf("  Agent:          %s\n", result.AgentType)
				fmt.Printf("  Confidence:     %.2f\n", result.ConfidenceScore)
				fmt.Printf("  Quality Score:  %.2f\n", result.QualityScore)
				fmt.Printf("  Execution Time: %dms\n", result.ExecutionTimeMs)
				fmt.Printf("  Created:        %s\n", result.CreatedAt.Format(time.RFC3339))
			}

			return nil
		},
	}

	cmd.Flags().StringVar(&format, "format", "table", "Output format (table, json)")

	return cmd
}

// Helper functions

func initializeBridge() (*bridge.CoordinationBridge, error) {
	dbPath := viper.GetString("db-path")

	config := &database.Config{
		CoordinationDBPath: dbPath + "/coordination.db",
		BeadsDBPath:        "./.beads/beads.db",
		TempoliteDBPath:    dbPath + "/tempolite.db",
	}

	dbManager, err := database.NewDatabaseManager(config)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize databases: %w", err)
	}

	beadsClient, err := beads.NewClient(dbManager.BeadsDBPath)
	if err != nil {
		return nil, fmt.Errorf("failed to create beads client: %w", err)
	}

	tempoliteEngine, err := tempolite.NewEngine()
	if err != nil {
		return nil, fmt.Errorf("failed to create tempolite engine: %w", err)
	}

	coordBridge := bridge.NewCoordinationBridge(beadsClient, tempoliteEngine, dbManager.CoordinationDB)

	return coordBridge, nil
}

func openCoordinationDB() (*sql.DB, error) {
	dbPath := viper.GetString("db-path")
	db, err := sql.Open("sqlite3", dbPath+"/coordination.db")
	if err != nil {
		return nil, err
	}
	return db, nil
}

func contains(slice []string, item string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}
	return false
}
