# SQLite Layer

The SQLite layer provides **fast queries and derived state** in Beads' three-layer architecture, delivering millisecond response times while maintaining complete rebuildability from JSONL.

## ⚡ Role in Three-Layer Architecture

The SQLite layer is Layer 3 in Beads' architecture:

```
┌─────────────────┐
│   Git Repo     │ ← Historical Source of Truth
│ (issues.jsonl) │
└────────┬────────┘
         │
┌────────▼────────┐
│   JSONL Files  │ ← Operational Source of Truth  
│ (append-only)  │
└────────┬────────┘
         │
┌────────▼────────┐
│   SQLite DB    │ ← Fast Queries / Derived State
│  (beads.db)   │
└─────────────────┘
```

**Key Characteristic**: SQLite provides *instant query performance* while being completely expendable and rebuildable.

## 🗃️ Database Structure

### Physical Files
```
.beads/
├── beads.db           # Main SQLite database (gitignored)
├── beads.db-shm      # Shared memory file (temporary)
├── beads.db-wal      # Write-Ahead Log (temporary)
└── .lock             # Database lock file
```

### Git-ignored Status
```
# .gitignore entry for Beads
.beads/beads.db*
.beads/.lock
.beads/.wisp/
```

**Rationale**: SQLite is derived state that can always be rebuilt from JSONL, making it safe to delete and recreate.

## 📊 Database Schema

### Core Tables

#### Issues Table
```sql
CREATE TABLE issues (
    id TEXT PRIMARY KEY,                    -- Hash-based ID (bd-a1b2)
    title TEXT NOT NULL,                     -- Issue title
    description TEXT,                       -- Detailed description
    status TEXT DEFAULT 'open',              -- open, in_progress, closed
    priority INTEGER DEFAULT 2,              -- 0 (highest) to 3 (lowest)
    type TEXT DEFAULT 'task',               -- task, bug, feature, epic
    assignee TEXT,                          -- Agent/user assigned
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    closed_at TIMESTAMP,                     -- When issue was closed
    closed_reason TEXT,                      -- Reason for closing
    parent_id TEXT,                         -- Parent issue (for hierarchy)
    FOREIGN KEY (parent_id) REFERENCES issues(id)
);

-- Indexes for performance
CREATE INDEX idx_issues_status ON issues(status);
CREATE INDEX idx_issues_priority ON issues(priority);
CREATE INDEX idx_issues_type ON issues(type);
CREATE INDEX idx_issues_assignee ON issues(assignee);
CREATE INDEX idx_issues_created_at ON issues(created_at);
CREATE INDEX idx_issues_parent_id ON issues(parent_id);
```

#### Dependencies Table
```sql
CREATE TABLE dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id TEXT NOT NULL,                -- Issue that blocks/is parent
    child_id TEXT NOT NULL,                 -- Issue that is blocked/child
    type TEXT NOT NULL,                     -- 'blocks', 'parent-child', 'discovered-from', 'related'
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_id) REFERENCES issues(id) ON DELETE CASCADE,
    FOREIGN KEY (child_id) REFERENCES issues(id) ON DELETE CASCADE,
    UNIQUE(parent_id, child_id, type)
);

-- Performance indexes
CREATE INDEX idx_deps_parent ON dependencies(parent_id);
CREATE INDEX idx_deps_child ON dependencies(child_id);
CREATE INDEX idx_deps_type ON dependencies(type);
```

#### Labels Table
```sql
CREATE TABLE labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL,                 -- Issue being labeled
    label TEXT NOT NULL,                     -- Label text
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE,
    UNIQUE(issue_id, label)
);

-- Performance index
CREATE INDEX idx_labels_issue_id ON labels(issue_id);
CREATE INDEX idx_labels_label ON labels(label);
```

#### Comments Table
```sql
CREATE TABLE comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL,                 -- Issue being commented on
    author TEXT NOT NULL,                    -- Comment author
    comment TEXT NOT NULL,                   -- Comment content
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);

-- Performance indexes
CREATE INDEX idx_comments_issue_id ON comments(issue_id);
CREATE INDEX idx_comments_created_at ON comments(created_at);
```

#### Metadata Table
```sql
CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- System metadata
INSERT INTO metadata (key, value) VALUES 
    ('schema_version', '1.0'),
    ('last_sync', '2026-02-06T10:00:00Z'),
    ('total_issues', '1247');
```

## 🔄 Query Performance

### Typical Query Patterns

#### List Issues with Filters
```sql
-- Fast: Uses multiple indexes
SELECT id, title, status, priority, type, created_at
FROM issues 
WHERE status = 'open' 
  AND priority IN (0, 1)
  AND type = 'bug'
ORDER BY priority ASC, created_at DESC;

-- Execution time: < 5ms for 10,000 issues
```

#### Dependency Queries
```sql
-- Find blocked issues
SELECT i.id, i.title, i.status
FROM issues i
JOIN dependencies d ON i.id = d.child_id
WHERE d.type = 'blocks' 
  AND d.parent_id IN (
    SELECT id FROM issues WHERE status != 'closed'
  );

-- Execution time: < 10ms for complex dependency graphs
```

#### Ready Work Calculation
```sql
-- Issues with no uncompleted blockers
SELECT i.id, i.title, i.priority
FROM issues i
WHERE i.status IN ('open', 'in_progress')
  AND NOT EXISTS (
    SELECT 1 FROM dependencies d
    JOIN issues p ON d.parent_id = p.id
    WHERE d.child_id = i.id 
      AND d.type = 'blocks'
      AND p.status != 'closed'
  )
ORDER BY i.priority ASC, i.created_at ASC;

-- Execution time: < 15ms for complex dependency graphs
```

#### Search Operations
```sql
-- Full-text search on title and description
SELECT id, title, 
       rank_bm25(title) + rank_bm25(description) as score
FROM issues_fts
WHERE title_fts MATCH 'database' OR description_fts MATCH 'database'
ORDER BY score DESC
LIMIT 20;

-- Requires FTS5 virtual table
-- Execution time: < 20ms for text search
```

## 🏗️ Database Rebuild Process

### Rebuild from JSONL
The SQLite database can always be rebuilt from JSONL:

```python
def rebuild_database(jsonl_file, db_path):
    """Complete SQLite rebuild from JSONL operations"""
    
    # 1. Create fresh database
    conn = sqlite3.connect(db_path)
    
    # 2. Initialize schema
    init_schema(conn)
    
    # 3. Process JSONL operations chronologically
    for line in jsonl_file:
        operation = json.loads(line)
        
        if operation['type'] == 'create':
            create_issue(conn, operation)
        elif operation['type'] == 'update':
            update_issue(conn, operation)
        elif operation['type'] == 'dependency':
            handle_dependency(conn, operation)
        elif operation['type'] == 'label':
            handle_label(conn, operation)
        elif operation['type'] == 'comment':
            handle_comment(conn, operation)
    
    # 4. Create indexes
    create_indexes(conn)
    
    # 5. Update metadata
    update_metadata(conn)
    
    conn.close()
```

### Rebuild Performance
```bash
# Database rebuild performance by size:

# Small repository (< 1,000 issues, < 5,000 operations)
Rebuild time: < 1 second
Memory usage: < 20MB
Peak disk I/O: < 10MB

# Medium repository (1,000-10,000 issues, 5,000-50,000 operations)  
Rebuild time: 1-5 seconds
Memory usage: 20-100MB
Peak disk I/O: 10-50MB

# Large repository (10,000+ issues, 50,000+ operations)
Rebuild time: 5-30 seconds
Memory usage: 100MB+
Peak disk I/O: 50MB+
```

### Atomic Rebuild Process
```bash
# Beads uses atomic rebuilds to prevent corruption:

1. Create temporary database: .beads/beads.db.new
2. Rebuild from JSONL into temporary file
3. Verify database integrity (PRAGMA integrity_check)
4. Atomic rename: beads.db.new -> beads.db
5. Update metadata table
```

## 🔧 Database Operations

### Connection Management
```go
// Beads connection pattern (simplified)
func OpenDatabase(path string) (*sql.DB, error) {
    db, err := sql.Open("sqlite3", path)
    if err != nil {
        return nil, err
    }
    
    // Performance optimizations
    db.SetMaxOpenConns(1)           // Single connection for SQLite
    db.SetMaxIdleConns(1)
    
    // SQLite pragmas for performance
    pragmas := []string{
        "PRAGMA journal_mode=WAL",      // Write-Ahead Logging
        "PRAGMA synchronous=NORMAL",     // Balanced safety/performance
        "PRAGMA cache_size=10000",     // 10MB cache
        "PRAGMA temp_store=MEMORY",     // Temporary tables in memory
        "PRAGMA foreign_keys=ON",       // Enable foreign key constraints
    }
    
    for _, pragma := range pragmas {
        db.Exec(pragma)
    }
    
    return db, nil
}
```

### Transaction Patterns
```go
// All write operations use transactions
func CreateIssue(db *sql.DB, issue Issue) error {
    tx, err := db.Begin()
    if err != nil {
        return err
    }
    defer tx.Rollback()
    
    // Insert issue
    _, err = tx.Exec(`
        INSERT INTO issues (id, title, description, priority, type, created_at)
        VALUES (?, ?, ?, ?, ?, ?)`,
        issue.ID, issue.Title, issue.Description, 
        issue.Priority, issue.Type, issue.CreatedAt)
    
    if err != nil {
        return err
    }
    
    // Insert labels if any
    for _, label := range issue.Labels {
        _, err = tx.Exec(`
            INSERT INTO labels (issue_id, label)
            VALUES (?, ?)`,
            issue.ID, label)
        if err != nil {
            return err
        }
    }
    
    return tx.Commit()
}
```

### Lock Management
```go
// Prevent concurrent database access
type Database struct {
    db    *sql.DB
    lock  sync.Mutex
}

func (d *Database) Execute(query string, args ...interface{}) error {
    d.lock.Lock()
    defer d.lock.Unlock()
    
    _, err := d.db.Exec(query, args...)
    return err
}
```

## 📈 Performance Optimization

### Index Strategy
```sql
-- Multi-column index for common queries
CREATE INDEX idx_issues_status_priority ON issues(status, priority);

-- Composite index for ready work calculation
CREATE INDEX idx_deps_blocks ON dependencies(child_id, type) 
WHERE type = 'blocks';

-- Full-text search index
CREATE VIRTUAL TABLE issues_fts USING fts5(
    title, 
    description,
    content='issues',
    content_rowid='rowid'
);
```

### Query Optimization
```sql
-- Use EXISTS instead of JOIN for existence checks
SELECT i.id, i.title
FROM issues i
WHERE i.status = 'open'
  AND NOT EXISTS (
    SELECT 1 FROM dependencies d
    WHERE d.child_id = i.id 
      AND d.type = 'blocks'
      AND d.parent_id IN (SELECT id FROM issues WHERE status != 'closed')
  );

-- Use LIMIT for large result sets
SELECT id, title, priority
FROM issues 
WHERE status = 'open'
ORDER BY priority ASC, created_at DESC
LIMIT 100;  -- Paginate for large sets
```

### Caching Strategies
```go
// Beads implements query result caching
type Cache struct {
    readyWork map[string][]Issue  // Cache by status filter
    mutex    sync.RWMutex
    ttl      time.Duration
}

func (c *Cache) GetReadyWork() []Issue {
    c.mutex.RLock()
    defer c.mutex.RUnlock()
    
    if cached, exists := c.cache["ready"]; exists {
        return cached
    }
    
    // Cache miss - query database
    issues := queryReadyWork()
    c.cache["ready"] = issues
    return issues
}
```

## 🔍 Database Analysis

### Performance Monitoring
```sql
-- Check database health
PRAGMA integrity_check;        -- Verify database integrity
PRAGMA table_info(issues);     -- Get table schema
PRAGMA index_list(issues);     -- List indexes
PRAGMA stats;                  -- Database statistics

-- Analyze query performance
EXPLAIN QUERY PLAN 
SELECT * FROM issues 
WHERE status = 'open' AND priority < 2;
```

### Size Analysis
```sql
-- Table sizes
SELECT 
    name,
    COUNT(*) as row_count,
    ROUND(SUM(LENGTH(sql)) / 1024.0, 2) as size_kb
FROM sqlite_master 
WHERE type = 'table'
GROUP BY name;

-- Index sizes  
SELECT 
    name,
    ROUND(SUM(LENGTH(sql)) / 1024.0, 2) as size_kb
FROM sqlite_master 
WHERE type = 'index'
GROUP BY name;
```

### Usage Patterns
```sql
-- Most common labels
SELECT label, COUNT(*) as issue_count
FROM labels
GROUP BY label
ORDER BY issue_count DESC
LIMIT 20;

-- Issue lifecycle metrics
SELECT 
    status,
    COUNT(*) as count,
    AVG(julianday('now') - julianday(created_at)) as days_open
FROM issues
GROUP BY status;

-- Dependency complexity
SELECT 
    i.id,
    COUNT(d.child_id) as blocks_count,
    COUNT(d.parent_id) as blocked_by_count
FROM issues i
LEFT JOIN dependencies d ON i.id = d.parent_id AND d.type = 'blocks'
LEFT JOIN dependencies p ON i.id = p.child_id AND p.type = 'blocks'
GROUP BY i.id
ORDER BY blocks_count DESC, blocked_by_count DESC;
```

## 🛡️ Database Corruption

### Corruption Detection
```bash
# Built-in corruption checking
bd check --database

# Manual SQLite checks
sqlite3 .beads/beads.db "PRAGMA integrity_check;"
sqlite3 .beads/beads.db "PRAGMA foreign_key_check;"
```

### Corruption Recovery
```bash
# Universal recovery sequence
rm .beads/beads.db*           # Remove corrupted database
bd sync --import-only          # Rebuild from JSONL

# Alternative: Manual SQLite recovery
sqlite3 .beads/beads.db ".recover" | sqlite3 .beads/beads.recovered.db
mv .beads/beads.recovered.db .beads/beads.db
```

### Corruption Prevention
```go
// Beads implements multiple corruption prevention measures

// 1. Write-Ahead Logging (WAL) for crash safety
db.Exec("PRAGMA journal_mode=WAL")

// 2. Regular integrity checks
func periodicCheck(db *sql.DB) {
    if time.Since(lastCheck) > time.Hour {
        db.Exec("PRAGMA integrity_check")
        lastCheck = time.Now()
    }
}

// 3. Atomic writes with temporary files
func atomicWrite(dbPath string, data []byte) error {
    tmpPath := dbPath + ".tmp"
    err := ioutil.WriteFile(tmpPath, data, 0644)
    if err != nil {
        return err
    }
    return os.Rename(tmpPath, dbPath)  // Atomic on Unix
}
```

## 🔗 Related Documentation

- [JSONL Layer](jsonl-layer.md) - Source of truth for SQLite
- [Git Layer](git-layer.md) - Historical storage layer
- [Data Flow](data-flow.md) - Complete system interaction
- [Recovery Overview](../recovery/) - Corruption recovery procedures
- [Performance](../best-practices/performance.md) - Optimization strategies

## 📚 See Also

- [Architecture Overview](overview.md) - Complete three-layer system
- [CLI Reference](../cli-reference/) - Commands that interact with SQLite
- [Extension Points](../extension-points/) - Direct database access patterns
- [Recovery Database Corruption](../recovery/database-corruption.md) - Specific recovery procedures