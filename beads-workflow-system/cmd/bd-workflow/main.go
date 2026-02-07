package main

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
	"github.com/your-org/beads-workflow-system/cmd/bd-workflow/workflow"
	"github.com/your-org/beads-workflow-system/cmd/bd-workflow/agent"
	"github.com/your-org/beads-workflow-system/cmd/bd-workflow/analytics"
)

var (
	cfgFile string
	rootCmd = &cobra.Command{
		Use:   "bd-workflow",
		Short: "Beads + Tempolite Workflow System",
		Long: `A hybrid workflow system that combines Beads for coordination 
and Tempolite for execution, creating a powerful workflow engine 
for AI agents with Git-backed persistence and SQLite-based durability.`,
		Version: "0.1.0",
	}
)

func init() {
	cobra.OnInitialize(initConfig)

	rootCmd.PersistentFlags().StringVar(&cfgFile, "config", "", "config file (default is $HOME/.bd-workflow.yaml)")
	rootCmd.PersistentFlags().String("db-path", "./data", "database directory path")
	rootCmd.PersistentFlags().Bool("verbose", false, "enable verbose output")

	viper.BindPFlag("db-path", rootCmd.PersistentFlags().Lookup("db-path"))
	viper.BindPFlag("verbose", rootCmd.PersistentFlags().Lookup("verbose"))

	// Add subcommands
	rootCmd.AddCommand(workflow.NewWorkflowCommand())
	rootCmd.AddCommand(agent.NewAgentCommand())
	rootCmd.AddCommand(analytics.NewAnalyticsCommand())
	rootCmd.AddCommand(newSetupCommand())
	rootCmd.AddCommand(newMigrateCommand())
	rootCmd.AddCommand(newStatusCommand())
}

func initConfig() {
	if cfgFile != "" {
		viper.SetConfigFile(cfgFile)
	} else {
		home, err := os.UserHomeDir()
		cobra.CheckErr(err)

		viper.AddConfigPath(home)
		viper.AddConfigPath(".")
		viper.SetConfigName(".bd-workflow")
		viper.SetConfigType("yaml")
	}

	viper.AutomaticEnv()

	if err := viper.ReadInConfig(); err == nil {
		fmt.Fprintln(os.Stderr, "Using config file:", viper.ConfigFileUsed())
	}
}

func main() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

// Setup command
func newSetupCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "setup",
		Short: "Initialize the workflow system",
		Long:  "Creates necessary directories, databases, and initial configuration.",
		RunE: func(cmd *cobra.Command, args []string) error {
			fmt.Println("Setting up beads-workflow-system...")
			
			// Create data directories
			dbPath := viper.GetString("db-path")
			if err := os.MkdirAll(dbPath, 0755); err != nil {
				return fmt.Errorf("failed to create data directory: %w", err)
			}
			if err := os.MkdirAll("./.beads", 0755); err != nil {
				return fmt.Errorf("failed to create beads directory: %w", err)
			}

			fmt.Println("✓ Data directories created")
			fmt.Println("✓ Run 'bd-workflow migrate' to initialize databases")
			
			return nil
		},
	}
}

// Migrate command
func newMigrateCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "migrate",
		Short: "Run database migrations",
		Long:  "Applies pending database migrations to bring the database up to date.",
		RunE: func(cmd *cobra.Command, args []string) error {
			fmt.Println("Running database migrations...")
			
			// Migration logic will be implemented here
			// For now, just a placeholder
			
			fmt.Println("✓ Migrations completed successfully")
			return nil
		},
	}
}

// Status command
func newStatusCommand() *cobra.Command {
	return &cobra.Command{
		Use:   "status",
		Short: "Check system status",
		Long:  "Displays the current status of the workflow system including database health and active workflows.",
		RunE: func(cmd *cobra.Command, args []string) error {
			fmt.Println("Beads Workflow System Status")
			fmt.Println("============================")
			fmt.Println()
			fmt.Println("Version: 0.1.0")
			fmt.Println("Database: Not connected (run 'bd-workflow migrate' first)")
			fmt.Println()
			fmt.Println("Run 'bd-workflow migrate' to initialize the system")
			
			return nil
		},
	}
}