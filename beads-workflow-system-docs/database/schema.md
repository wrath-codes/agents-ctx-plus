# Database Schema Documentation

## Complete SQL Schema

### 1. Core Tables

#### workflow_mappings
Bridges beads issues with tempolite workflows.

```sql
CREATE TABLE IF NOT EXISTS workflow_mappings (
    beads_issue_id TEXT PRIMARY KEY,
    tempolite_workflow_id TEXT NOT NULL UNIQUE,
    workflow_type TEXT NOT NULL CHECK (
        workflow_type IN ('research', 'poc', 'documentation', 'validation', 'supervisor')
    ),
    status TEXT DEFAULT 'active' CHECK (
        status IN ('active', 'completed', 'failed', 'paused', 'cancelled')
    ),
    priority INTEGER DEFAULT 2 CHECK (priority BETWEEN 0 AND 3),
    metadata JSON,
    parent_workflow_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    FOREIGN KEY (parent_workflow_id) REFERENCES workflow_mappings(beads_issue_id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_workflow_mappings_tempolite_id 
ON workflow_mappings(tempolite_workflow_id);

CREATE INDEX IF NOT EXISTS idx_workflow_mappings_type_status 
ON workflow_mappings(workflow_type, status);

CREATE INDEX IF NOT EXISTS idx_workflow_mappings_status_priority 
ON workflow_mappings(status, priority);

CREATE INDEX IF NOT EXISTS idx_workflow_mappings_parent 
ON workflow_mappings(parent_workflow_id);
```

#### agent_assignments
Tracks agent assignments to workflows.

```sql
CREATE TABLE IF NOT EXISTS agent_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL CHECK (
        agent_type IN ('research', 'poc', 'documentation', 'validation', 'supervisor')
    ),
    agent_id TEXT NOT NULL,
    step_number INTEGER DEFAULT 1,
    step_name TEXT,
    assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    status TEXT DEFAULT 'assigned' CHECK (
        status IN ('assigned', 'started', 'completed', 'failed', 'cancelled')
    ),
    handoff_from TEXT,
    handoff_to TEXT,
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_agent_assignments_agent_status 
ON agent_assignments(agent_id, status);

CREATE INDEX IF NOT EXISTS idx_agent_assignments_workflow 
ON agent_assignments(workflow_id, step_number);

CREATE INDEX IF NOT EXISTS idx_agent_assignments_agent_type 
ON agent_assignments(agent_type, status);
```

#### workflow_results
Stores execution results.

```sql
CREATE TABLE IF NOT EXISTS workflow_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    result_type TEXT NOT NULL,
    result_data JSON NOT NULL,
    confidence_score REAL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    execution_time_ms INTEGER,
    artifacts JSON,
    metadata JSON,
    quality_score REAL CHECK (quality_score >= 0.0 AND quality_score <= 10.0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_workflow_results_workflow 
ON workflow_results(workflow_id);

CREATE INDEX IF NOT EXISTS idx_workflow_results_agent_type 
ON workflow_results(agent_type, result_type);
```

### 2. Schema Migrations

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    name TEXT NOT NULL,
    checksum TEXT NOT NULL
);
```

### 3. SQLite Optimizations

```sql
-- Enable WAL mode for better concurrency
PRAGMA journal_mode = WAL;

-- Optimize for performance
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = 10000;
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = 268435456;
PRAGMA wal_autocheckpoint = 1000;
PRAGMA foreign_keys = ON;
```