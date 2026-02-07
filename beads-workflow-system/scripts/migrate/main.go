package main

import (
	"database/sql"
	"fmt"
	"os"

	_ "github.com/mattn/go-sqlite3"
	"github.com/your-org/beads-workflow-system/internal/migrations"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: go run scripts/migrate.go [command]")
		fmt.Println("Commands:")
		fmt.Println("  migrate    - Run pending migrations")
		fmt.Println("  status     - Show migration status")
		fmt.Println("  create     - Create a new migration")
		os.Exit(1)
	}

	command := os.Args[1]

	switch command {
	case "migrate":
		runMigrations()
	case "status":
		showStatus()
	case "create":
		if len(os.Args) < 3 {
			fmt.Println("Usage: go run scripts/migrate.go create <migration_name>")
			os.Exit(1)
		}
		createMigration(os.Args[2])
	default:
		fmt.Printf("Unknown command: %s\n", command)
		os.Exit(1)
	}
}

func runMigrations() {
	// Create directories
	os.MkdirAll("./data", 0755)
	os.MkdirAll("./.beads", 0755)

	// Apply to coordination database
	fmt.Println("Applying migrations to coordination database...")
	coordDB, err := sql.Open("sqlite3", "./data/coordination.db")
	if err != nil {
		fmt.Printf("Failed to open coordination DB: %v\n", err)
		os.Exit(1)
	}
	defer coordDB.Close()

	migrator := migrations.NewMigrator(coordDB, "./migrations")
	if err := migrator.Migrate(); err != nil {
		fmt.Printf("Migration failed: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("✓ Coordination database migrated")

	// Apply to tempolite database
	fmt.Println("Applying migrations to tempolite database...")
	tempDB, err := sql.Open("sqlite3", "./data/tempolite.db")
	if err != nil {
		fmt.Printf("Failed to open tempolite DB: %v\n", err)
		os.Exit(1)
	}
	defer tempDB.Close()

	tempMigrator := migrations.NewMigrator(tempDB, "./migrations")
	if err := tempMigrator.Migrate(); err != nil {
		fmt.Printf("Migration failed: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("✓ Tempolite database migrated")

	// Apply to beads database
	fmt.Println("Applying migrations to beads database...")
	beadsDB, err := sql.Open("sqlite3", "./.beads/beads.db")
	if err != nil {
		fmt.Printf("Failed to open beads DB: %v\n", err)
		os.Exit(1)
	}
	defer beadsDB.Close()

	beadsMigrator := migrations.NewMigrator(beadsDB, "./migrations")
	if err := beadsMigrator.Migrate(); err != nil {
		fmt.Printf("Migration failed: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("✓ Beads database migrated")

	fmt.Println("\n✓ All migrations completed successfully!")
}

func showStatus() {
	coordDB, err := sql.Open("sqlite3", "./data/coordination.db")
	if err != nil {
		fmt.Printf("Failed to open coordination DB: %v\n", err)
		os.Exit(1)
	}
	defer coordDB.Close()

	migrator := migrations.NewMigrator(coordDB, "./migrations")
	status, err := migrator.Status()
	if err != nil {
		fmt.Printf("Failed to get status: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Migration Status:\n")
	fmt.Printf("  Total:   %d\n", status.Total)
	fmt.Printf("  Applied: %d\n", status.Applied)
	fmt.Printf("  Pending: %d\n", status.Pending)

	if len(status.PendingMigrations) > 0 {
		fmt.Printf("\nPending migrations:\n")
		for _, m := range status.PendingMigrations {
			fmt.Printf("  - %s: %s\n", m.Version, m.Description)
		}
	}
}

func createMigration(name string) {
	filepath, err := migrations.CreateMigration("./migrations", name)
	if err != nil {
		fmt.Printf("Failed to create migration: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("✓ Created migration: %s\n", filepath)
}