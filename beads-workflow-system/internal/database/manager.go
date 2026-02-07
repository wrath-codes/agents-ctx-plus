package database

import (
	"database/sql"
	"fmt"
	"os"
	"time"

	_ "github.com/mattn/go-sqlite3"
)

// DatabaseManager manages all database connections
type DatabaseManager struct {
	CoordinationDB *sql.DB
	BeadsDB        *sql.DB
	TempoliteDB    *sql.DB
	BeadsDBPath    string // Path to beads.db for the real beads client
}

// Config holds database configuration
type Config struct {
	CoordinationDBPath string
	BeadsDBPath        string
	TempoliteDBPath    string
	MaxOpenConns       int
	MaxIdleConns       int
	ConnMaxLifetime    time.Duration
}

// DefaultConfig returns default database configuration
func DefaultConfig() *Config {
	return &Config{
		CoordinationDBPath: "./data/coordination.db",
		BeadsDBPath:        "./.beads/beads.db",
		TempoliteDBPath:    "./data/tempolite.db",
		MaxOpenConns:       1, // SQLite supports only 1 writer
		MaxIdleConns:       1,
		ConnMaxLifetime:    time.Hour,
	}
}

// NewDatabaseManager creates a new database manager
func NewDatabaseManager(config *Config) (*DatabaseManager, error) {
	if config == nil {
		config = DefaultConfig()
	}

	// Open coordination database
	coordDB, err := openDB(config.CoordinationDBPath, config)
	if err != nil {
		return nil, fmt.Errorf("failed to open coordination database: %w", err)
	}

	// Open beads database (may not exist yet)
	var beadsDB *sql.DB
	if fileExists(config.BeadsDBPath) {
		beadsDB, err = openDB(config.BeadsDBPath, config)
		if err != nil {
			coordDB.Close()
			return nil, fmt.Errorf("failed to open beads database: %w", err)
		}
	}

	// Open tempolite database
	tempoliteDB, err := openDB(config.TempoliteDBPath, config)
	if err != nil {
		coordDB.Close()
		if beadsDB != nil {
			beadsDB.Close()
		}
		return nil, fmt.Errorf("failed to open tempolite database: %w", err)
	}

	return &DatabaseManager{
		CoordinationDB: coordDB,
		BeadsDB:        beadsDB,
		TempoliteDB:    tempoliteDB,
		BeadsDBPath:    config.BeadsDBPath,
	}, nil
}

// openDB opens a SQLite database with optimized settings
func openDB(path string, config *Config) (*sql.DB, error) {
	db, err := sql.Open("sqlite3", path+"?_journal_mode=WAL&_synchronous=NORMAL&_cache_size=10000&_temp_store=MEMORY")
	if err != nil {
		return nil, err
	}

	db.SetMaxOpenConns(config.MaxOpenConns)
	db.SetMaxIdleConns(config.MaxIdleConns)
	db.SetConnMaxLifetime(config.ConnMaxLifetime)

	// Test connection
	if err := db.Ping(); err != nil {
		db.Close()
		return nil, err
	}

	// Enable foreign keys (must be done per-connection for SQLite)
	if _, err := db.Exec("PRAGMA foreign_keys = ON"); err != nil {
		db.Close()
		return nil, fmt.Errorf("failed to enable foreign keys: %w", err)
	}

	return db, nil
}

// Close closes all database connections
func (dm *DatabaseManager) Close() error {
	var errs []error

	if dm.CoordinationDB != nil {
		if err := dm.CoordinationDB.Close(); err != nil {
			errs = append(errs, err)
		}
	}

	if dm.BeadsDB != nil {
		if err := dm.BeadsDB.Close(); err != nil {
			errs = append(errs, err)
		}
	}

	if dm.TempoliteDB != nil {
		if err := dm.TempoliteDB.Close(); err != nil {
			errs = append(errs, err)
		}
	}

	if len(errs) > 0 {
		return fmt.Errorf("errors closing databases: %v", errs)
	}

	return nil
}

// fileExists checks if a file exists on disk
func fileExists(path string) bool {
	_, err := os.Stat(path)
	return !os.IsNotExist(err)
}

// HealthCheck checks database health
func (dm *DatabaseManager) HealthCheck() error {
	if err := dm.CoordinationDB.Ping(); err != nil {
		return fmt.Errorf("coordination database unhealthy: %w", err)
	}

	if dm.BeadsDB != nil {
		if err := dm.BeadsDB.Ping(); err != nil {
			return fmt.Errorf("beads database unhealthy: %w", err)
		}
	}

	if err := dm.TempoliteDB.Ping(); err != nil {
		return fmt.Errorf("tempolite database unhealthy: %w", err)
	}

	return nil
}

// BeginCoordinationTx begins a transaction on the coordination database
func (dm *DatabaseManager) BeginCoordinationTx() (*sql.Tx, error) {
	return dm.CoordinationDB.Begin()
}

// BeginBeadsTx begins a transaction on the beads database
func (dm *DatabaseManager) BeginBeadsTx() (*sql.Tx, error) {
	if dm.BeadsDB == nil {
		return nil, fmt.Errorf("beads database not available")
	}
	return dm.BeadsDB.Begin()
}

// BeginTempoliteTx begins a transaction on the tempolite database
func (dm *DatabaseManager) BeginTempoliteTx() (*sql.Tx, error) {
	return dm.TempoliteDB.Begin()
}
