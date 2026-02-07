package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: go run scripts/create_migration.go <migration_name>")
		os.Exit(1)
	}

	name := strings.Join(os.Args[1:], "_")
	migrationsDir := "./migrations"

	// Generate timestamp-based version
	version := time.Now().Format("20060102150405")
	
	// Sanitize name
	name = strings.ToLower(name)
	name = strings.ReplaceAll(name, " ", "_")
	name = strings.ReplaceAll(name, "-", "_")
	
	filename := fmt.Sprintf("%s_%s.sql", version, name)
	filepath := filepath.Join(migrationsDir, filename)
	
	// Create file with template
	content := fmt.Sprintf(`-- Migration: %s
-- Version: %s
-- Created: %s

-- Up migration

-- Down migration (not supported in SQLite)
`, name, version, time.Now().Format("2006-01-02 15:04:05"))
	
	if err := os.WriteFile(filepath, []byte(content), 0644); err != nil {
		fmt.Printf("Error creating migration: %v\n", err)
		os.Exit(1)
	}
	
	fmt.Printf("✓ Created migration: %s\n", filepath)
}