package database

import (
	"os"
	"path/filepath"
	"testing"
)

func TestFileExistsTrue(t *testing.T) {
	// Create a temp file
	dir := t.TempDir()
	path := filepath.Join(dir, "test.db")
	if err := os.WriteFile(path, []byte("test"), 0644); err != nil {
		t.Fatal(err)
	}

	if !fileExists(path) {
		t.Error("fileExists returned false for existing file")
	}
}

func TestFileExistsFalse(t *testing.T) {
	if fileExists("/nonexistent/path/does/not/exist.db") {
		t.Error("fileExists returned true for nonexistent file")
	}
}

func TestNewDatabaseManager(t *testing.T) {
	dir := t.TempDir()

	config := &Config{
		CoordinationDBPath: filepath.Join(dir, "coord.db"),
		BeadsDBPath:        filepath.Join(dir, "nonexistent-beads.db"), // doesn't exist
		TempoliteDBPath:    filepath.Join(dir, "tempolite.db"),
		MaxOpenConns:       1,
		MaxIdleConns:       1,
	}

	dm, err := NewDatabaseManager(config)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	defer dm.Close()

	// Beads DB should be nil since file doesn't exist
	if dm.BeadsDB != nil {
		t.Error("expected BeadsDB to be nil when file doesn't exist")
	}

	// Coordination and tempolite should be open
	if dm.CoordinationDB == nil {
		t.Error("expected CoordinationDB to be non-nil")
	}
	if dm.TempoliteDB == nil {
		t.Error("expected TempoliteDB to be non-nil")
	}
}

func TestDatabaseManagerHealthCheck(t *testing.T) {
	dir := t.TempDir()

	config := &Config{
		CoordinationDBPath: filepath.Join(dir, "coord.db"),
		BeadsDBPath:        filepath.Join(dir, "nonexistent.db"),
		TempoliteDBPath:    filepath.Join(dir, "tempolite.db"),
		MaxOpenConns:       1,
		MaxIdleConns:       1,
	}

	dm, err := NewDatabaseManager(config)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	defer dm.Close()

	if err := dm.HealthCheck(); err != nil {
		t.Errorf("health check failed: %v", err)
	}
}

func TestDatabaseManagerClose(t *testing.T) {
	dir := t.TempDir()

	config := &Config{
		CoordinationDBPath: filepath.Join(dir, "coord.db"),
		BeadsDBPath:        filepath.Join(dir, "nonexistent.db"),
		TempoliteDBPath:    filepath.Join(dir, "tempolite.db"),
		MaxOpenConns:       1,
		MaxIdleConns:       1,
	}

	dm, err := NewDatabaseManager(config)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if err := dm.Close(); err != nil {
		t.Errorf("close failed: %v", err)
	}
}