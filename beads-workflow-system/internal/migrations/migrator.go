package migrations

import (
	"crypto/sha256"
	"database/sql"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

// Migration represents a database migration
type Migration struct {
	Version     string
	Description string
	FilePath    string
	UpSQL       string
	Checksum    string
}

// Migrator handles database migrations
type Migrator struct {
	db          *sql.DB
	migrationsDir string
}

// NewMigrator creates a new migrator instance
func NewMigrator(db *sql.DB, migrationsDir string) *Migrator {
	return &Migrator{
		db:            db,
		migrationsDir: migrationsDir,
	}
}

// Init creates the schema_migrations table
func (m *Migrator) Init() error {
	_, err := m.db.Exec(`
		CREATE TABLE IF NOT EXISTS schema_migrations (
			version TEXT PRIMARY KEY,
			applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			checksum TEXT NOT NULL,
			description TEXT,
			execution_time_ms INTEGER,
			success BOOLEAN DEFAULT TRUE
		)
	`)
	return err
}

// LoadMigrations loads all migrations from the migrations directory
func (m *Migrator) LoadMigrations() ([]*Migration, error) {
	entries, err := os.ReadDir(m.migrationsDir)
	if err != nil {
		return nil, fmt.Errorf("failed to read migrations directory: %w", err)
	}

	var migrations []*Migration
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}

		name := entry.Name()
		if !strings.HasSuffix(name, ".sql") {
			continue
		}

		// Extract version from filename (e.g., 001_initial_schema.sql -> 001)
		parts := strings.SplitN(name, "_", 2)
		if len(parts) < 2 {
			continue
		}
		version := parts[0]

		// Read file content
		filePath := filepath.Join(m.migrationsDir, name)
		content, err := os.ReadFile(filePath)
		if err != nil {
			return nil, fmt.Errorf("failed to read migration file %s: %w", name, err)
		}

		// Calculate checksum
		hash := sha256.Sum256(content)
		checksum := hex.EncodeToString(hash[:])

		// Extract description from filename
		desc := strings.TrimSuffix(parts[1], ".sql")
		desc = strings.ReplaceAll(desc, "_", " ")

		migrations = append(migrations, &Migration{
			Version:     version,
			Description: desc,
			FilePath:    filePath,
			UpSQL:       string(content),
			Checksum:    checksum,
		})
	}

	// Sort by version
	sort.Slice(migrations, func(i, j int) bool {
		return migrations[i].Version < migrations[j].Version
	})

	return migrations, nil
}

// GetAppliedMigrations returns all migrations that have been applied
func (m *Migrator) GetAppliedMigrations() (map[string]bool, error) {
	rows, err := m.db.Query("SELECT version FROM schema_migrations WHERE success = TRUE")
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	applied := make(map[string]bool)
	for rows.Next() {
		var version string
		if err := rows.Scan(&version); err != nil {
			return nil, err
		}
		applied[version] = true
	}

	return applied, rows.Err()
}

// Migrate applies all pending migrations
func (m *Migrator) Migrate() error {
	// Ensure migrations table exists
	if err := m.Init(); err != nil {
		return fmt.Errorf("failed to initialize migrations table: %w", err)
	}

	// Load all migrations
	migrations, err := m.LoadMigrations()
	if err != nil {
		return err
	}

	if len(migrations) == 0 {
		return nil
	}

	// Get applied migrations
	applied, err := m.GetAppliedMigrations()
	if err != nil {
		return fmt.Errorf("failed to get applied migrations: %w", err)
	}

	// Apply pending migrations
	for _, migration := range migrations {
		if applied[migration.Version] {
			continue
		}

		if err := m.applyMigration(migration); err != nil {
			return fmt.Errorf("failed to apply migration %s: %w", migration.Version, err)
		}
	}

	return nil
}

// applyMigration applies a single migration
func (m *Migrator) applyMigration(migration *Migration) error {
	startTime := time.Now()

	tx, err := m.db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()

	// Execute migration SQL
	if _, err := tx.Exec(migration.UpSQL); err != nil {
		return fmt.Errorf("migration execution failed: %w", err)
	}

	// Record migration
	executionTime := time.Since(startTime).Milliseconds()
	_, err = tx.Exec(
		`INSERT INTO schema_migrations (version, checksum, description, execution_time_ms, success) 
		 VALUES (?, ?, ?, ?, TRUE)`,
		migration.Version, migration.Checksum, migration.Description, executionTime,
	)
	if err != nil {
		return fmt.Errorf("failed to record migration: %w", err)
	}

	return tx.Commit()
}

// Status returns the current migration status
func (m *Migrator) Status() (*MigrationStatus, error) {
	migrations, err := m.LoadMigrations()
	if err != nil {
		return nil, err
	}

	applied, err := m.GetAppliedMigrations()
	if err != nil {
		return nil, err
	}

	var pending []*Migration
	var completed []*Migration

	for _, m := range migrations {
		if applied[m.Version] {
			completed = append(completed, m)
		} else {
			pending = append(pending, m)
		}
	}

	return &MigrationStatus{
		Total:     len(migrations),
		Applied:   len(completed),
		Pending:   len(pending),
		Completed: completed,
		PendingMigrations: pending,
	}, nil
}

// MigrationStatus represents the migration status
type MigrationStatus struct {
	Total             int
	Applied           int
	Pending           int
	Completed         []*Migration
	PendingMigrations []*Migration
}

// Rollback rolls back the last migration (not implemented for SQLite)
func (m *Migrator) Rollback() error {
	return fmt.Errorf("rollback not supported in SQLite - use database backup/restore instead")
}

// CreateMigration creates a new migration file
func CreateMigration(migrationsDir, name string) (string, error) {
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
		return "", err
	}
	
	return filepath, nil
}

// VerifyChecksums verifies that applied migrations haven't been modified
func (m *Migrator) VerifyChecksums() error {
	migrations, err := m.LoadMigrations()
	if err != nil {
		return err
	}

	rows, err := m.db.Query("SELECT version, checksum FROM schema_migrations WHERE success = TRUE")
	if err != nil {
		return err
	}
	defer rows.Close()

	appliedChecksums := make(map[string]string)
	for rows.Next() {
		var version, checksum string
		if err := rows.Scan(&version, &checksum); err != nil {
			return err
		}
		appliedChecksums[version] = checksum
	}

	for _, migration := range migrations {
		if appliedChecksum, exists := appliedChecksums[migration.Version]; exists {
			if appliedChecksum != migration.Checksum {
				return fmt.Errorf("migration %s has been modified (checksum mismatch)", migration.Version)
			}
		}
	}

	return nil
}

// calculateChecksum calculates SHA256 checksum of a file
func calculateChecksum(filePath string) (string, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return "", err
	}
	defer file.Close()

	hash := sha256.New()
	if _, err := io.Copy(hash, file); err != nil {
		return "", err
	}

	return hex.EncodeToString(hash.Sum(nil)), nil
}