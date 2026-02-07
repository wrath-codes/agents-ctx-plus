# Database Schema Design

## 🗄️ Database Architecture Overview

The workflow system uses **three separate SQLite databases** with specific responsibilities:

1. **Beads Database** (`.beads/beads.db`) - Core issue tracking
2. **Tempolite Database** (`tempolite.db`) - Workflow execution engine
3. **Coordination Database** (`coordination.db`) - Bridge between systems

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Beads DB      │    │  Tempolite DB  │    │ Coordination DB │
│                 │    │                 │    │                 │
│ • Issues       │    │ • Workflows    │    │ • Mappings     │
│ • Dependencies  │    │ • Activities    │    │ • Assignments   │
│ • Comments     │    │ • Sagas         │    │ • Results       │
│ • Labels       │    │ • Signals       │    │ • Performance   │
│ • Metadata     │    │ • Checkpoints   │    │ • Analytics     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┴───────────────────────┘
                            ┌─────────────────────┐
                            │  Application Layer │
                            └─────────────────────┘
```

## 📋 1. Coordination Database Schema

### Core Mapping Tables

#### workflow_mappings
Bridges beads issues to tempolite workflows.

```sql
CREATE TABLE workflow_mappings (
    beads_issue_id TEXT PRIMARY KEY,
    tempolite_workflow_id TEXT NOT NULL UNIQUE,
    workflow_type TEXT NOT NULL CHECK (workflow_type IN ('research', 'poc', 'documentation', 'validation')),
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'completed', 'failed', 'paused', 'cancelled')),
    priority INTEGER DEFAULT 2 CHECK (priority BETWEEN 0 AND 3),
    metadata JSON,
    parent_workflow_id TEXT, -- For nested workflows
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    
    FOREIGN KEY (parent_workflow_id) REFERENCES workflow_mappings(beads_issue_id)
);

-- Indexes for performance
CREATE INDEX idx_workflow_mappings_tempolite_id ON workflow_mappings(tempolite_workflow_id);
CREATE INDEX idx_workflow_mappings_type_status ON workflow_mappings(workflow_type, status);
CREATE INDEX idx_workflow_mappings_status_priority ON workflow_mappings(status, priority);
CREATE INDEX idx_workflow_mappings_parent ON workflow_mappings(parent_workflow_id);
```

#### agent_assignments
Tracks which agent is working on which workflow step.

```sql
CREATE TABLE agent_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL CHECK (agent_type IN ('research', 'poc', 'documentation', 'validation', 'supervisor')),
    agent_id TEXT NOT NULL, -- Unique agent identifier
    step_number INTEGER DEFAULT 1,
    step_name TEXT,
    assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    status TEXT DEFAULT 'assigned' CHECK (status IN ('assigned', 'started', 'completed', 'failed', 'cancelled')),
    handoff_from TEXT,
    handoff_to TEXT,
    
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes
CREATE INDEX idx_agent_assignments_agent_status ON agent_assignments(agent_id, status);
CREATE INDEX idx_agent_assignments_workflow ON agent_assignments(workflow_id, step_number);
CREATE INDEX idx_agent_assignments_agent_type ON agent_assignments(agent_type, status);
```

### Results Storage Tables

#### workflow_results
Stores structured results from each workflow phase.

```sql
CREATE TABLE workflow_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    result_type TEXT NOT NULL, -- findings, poc_results, documentation, validation, performance
    result_data JSON NOT NULL,
    confidence_score REAL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    execution_time_ms INTEGER,
    artifacts JSON, -- Paths to generated files
    metadata JSON,
    quality_score REAL CHECK (quality_score >= 0.0 AND quality_score <= 10.0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes
CREATE INDEX idx_workflow_results_workflow ON workflow_results(workflow_id);
CREATE INDEX idx_workflow_results_agent_type ON workflow_results(agent_type, result_type);
CREATE INDEX idx_workflow_results_quality ON workflow_results(quality_score DESC);
CREATE INDEX idx_workflow_results_created ON workflow_results(created_at DESC);
```

#### research_findings
Structured storage for research-specific results.

```sql
CREATE TABLE research_findings (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    library_name TEXT NOT NULL,
    library_version TEXT,
    documentation_url TEXT,
    findings JSON NOT NULL,
    confidence_score REAL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    relevance_score REAL CHECK (relevance_score >= 0.0 AND relevance_score <= 1.0),
    analysis_method TEXT, -- static_analysis, documentation_review, benchmark
    file_paths JSON, -- Downloaded/analyzed files
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes
CREATE INDEX idx_research_findings_workflow ON research_findings(workflow_id);
CREATE INDEX idx_research_findings_library ON research_findings(library_name, library_version);
CREATE INDEX idx_research_findings_confidence ON research_findings(confidence_score DESC);
```

#### poc_results
Detailed storage for proof-of-concept results.

```sql
CREATE TABLE poc_results (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    implementation_type TEXT NOT NULL, -- function, module, service, full_application
    language TEXT NOT NULL,
    framework TEXT,
    build_success BOOLEAN DEFAULT FALSE,
    test_success BOOLEAN DEFAULT FALSE,
    performance_metrics JSON, -- latency, throughput, memory_usage, etc.
    test_results JSON,
    artifacts JSON, -- Generated code, binaries, etc.
    benchmarks JSON,
    error_message TEXT,
    execution_time_ms INTEGER,
    complexity_score REAL CHECK (complexity_score >= 0.0 AND complexity_score <= 10.0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes
CREATE INDEX idx_poc_results_workflow ON poc_results(workflow_id);
CREATE INDEX idx_poc_results_success ON poc_results(build_success, test_success);
CREATE INDEX idx_poc_results_performance ON poc_results(execution_time_ms DESC);
```

### Performance and Analytics Tables

#### workflow_performance
Detailed performance metrics for optimization.

```sql
CREATE TABLE workflow_performance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    step_name TEXT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    duration_ms INTEGER,
    success BOOLEAN DEFAULT FALSE,
    error_message TEXT,
    memory_usage_mb INTEGER,
    cpu_usage_percent REAL,
    disk_io_bytes INTEGER,
    network_io_bytes INTEGER,
    resource_metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes
CREATE INDEX idx_workflow_performance_workflow ON workflow_performance(workflow_id);
CREATE INDEX idx_workflow_performance_duration ON workflow_performance(duration_ms DESC);
CREATE INDEX idx_workflow_performance_success ON workflow_performance(success, duration_ms);
```

#### workflow_analytics
Aggregated analytics for reporting and insights.

```sql
CREATE TABLE workflow_analytics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date DATE NOT NULL,
    workflow_type TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    total_workflows INTEGER DEFAULT 0,
    successful_workflows INTEGER DEFAULT 0,
    failed_workflows INTEGER DEFAULT 0,
    avg_execution_time_ms INTEGER,
    avg_confidence_score REAL,
    total_agents_active INTEGER DEFAULT 0,
    system_load_avg REAL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE(date, workflow_type, agent_type)
);

-- Indexes
CREATE INDEX idx_workflow_analytics_date ON workflow_analytics(date DESC);
CREATE INDEX idx_workflow_analytics_type ON workflow_analytics(workflow_type, agent_type);
```

### Configuration and Template Tables

#### workflow_templates
Reusable workflow templates and formulas.

```sql
CREATE TABLE workflow_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    workflow_type TEXT NOT NULL,
    template_def JSON NOT NULL, -- Template definition
    agent_sequence JSON NOT NULL, -- Ordered list of agent types
    variables JSON, -- Template variables
    version TEXT DEFAULT '1.0',
    is_active BOOLEAN DEFAULT TRUE,
    usage_count INTEGER DEFAULT 0,
    success_rate REAL DEFAULT 0.0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_workflow_templates_type ON workflow_templates(workflow_type);
CREATE INDEX idx_workflow_templates_active ON workflow_templates(is_active, usage_count DESC);
```

#### agent_configurations
Agent-specific configurations and capabilities.

```sql
CREATE TABLE agent_configurations (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    configuration JSON NOT NULL,
    capabilities JSON, -- What the agent can do
    max_workload INTEGER DEFAULT 5,
    current_workload INTEGER DEFAULT 0,
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'busy', 'error')),
    last_heartbeat TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    performance_metrics JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE(agent_type, agent_id)
);

-- Indexes
CREATE INDEX idx_agent_configurations_type_status ON agent_configurations(agent_type, status);
CREATE INDEX idx_agent_configurations_workload ON agent_configurations(current_workload, max_workload);
```

## 📋 2. Integration Views

### Comprehensive Workflow View

```sql
CREATE VIEW workflow_overview AS
SELECT 
    wm.beads_issue_id,
    wm.tempolite_workflow_id,
    wm.workflow_type,
    wm.status as workflow_status,
    wm.priority,
    wm.created_at as workflow_created,
    aa.agent_id as assigned_agent,
    aa.agent_type as assigned_agent_type,
    aa.step_number,
    aa.status as assignment_status,
    (SELECT COUNT(*) FROM workflow_results wr WHERE wr.workflow_id = wm.tempolite_workflow_id) as result_count,
    (SELECT MAX(confidence_score) FROM workflow_results wr WHERE wr.workflow_id = wm.tempolite_workflow_id) as max_confidence,
    (SELECT AVG(execution_time_ms) FROM workflow_performance wp WHERE wp.workflow_id = wm.tempolite_workflow_id) as avg_execution_time
FROM workflow_mappings wm
LEFT JOIN agent_assignments aa ON wm.tempolite_workflow_id = aa.workflow_id AND aa.status IN ('assigned', 'started', 'completed')
WHERE wm.status != 'cancelled';
```

### Agent Workload View

```sql
CREATE VIEW agent_workload AS
SELECT 
    ac.agent_id,
    ac.agent_type,
    ac.status as agent_status,
    ac.current_workload,
    ac.max_workload,
    CAST(ac.current_workload AS REAL) / ac.max_workload as workload_percentage,
    (SELECT COUNT(*) FROM agent_assignments aa 
     WHERE aa.agent_id = ac.agent_id AND aa.status IN ('assigned', 'started')) as active_assignments,
    ac.last_heartbeat
FROM agent_configurations ac
WHERE ac.status = 'active';
```

### Performance Analytics View

```sql
CREATE VIEW performance_trends AS
SELECT 
    DATE(wp.created_at) as performance_date,
    wp.agent_type,
    COUNT(*) as total_executions,
    AVG(wp.duration_ms) as avg_duration_ms,
    SUM(CASE WHEN wp.success = 1 THEN 1 ELSE 0 END) as successful_executions,
    AVG(wp.memory_usage_mb) as avg_memory_mb,
    AVG(wp.cpu_usage_percent) as avg_cpu_percent
FROM workflow_performance wp
WHERE wp.created_at >= DATE('now', '-30 days')
GROUP BY DATE(wp.created_at), wp.agent_type
ORDER BY performance_date DESC, wp.agent_type;
```

## 🔧 Database Optimization

### SQLite Pragmas for Performance

```sql
-- Enable Write-Ahead Logging for better concurrency
PRAGMA journal_mode = WAL;

-- Optimize for performance
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = 10000; -- 10MB cache
PRAGMA temp_store = MEMORY;
PRAGMA wal_autocheckpoint = 1000;

-- Optimize for SSD storage
PRAGMA mmap_size = 268435456; -- 256MB memory mapping

-- Foreign key constraints
PRAGMA foreign_keys = ON;

-- Query optimizer hints
PRAGMA optimize = 0x10002; -- Enable all optimizations
```

### Connection Pooling Configuration

```go
type DatabaseConfig struct {
    CoordinationDB struct {
        MaxOpenConns    int           `yaml:"max_open_conns"`
        MaxIdleConns   int           `yaml:"max_idle_conns"`
        ConnMaxLifetime time.Duration `yaml:"conn_max_lifetime"`
        BusyTimeout     time.Duration `yaml:"busy_timeout"`
    } `yaml:"coordination_db"`
    
    TempoliteDB struct {
        MaxOpenConns    int           `yaml:"max_open_conns"`
        MaxIdleConns   int           `yaml:"max_idle_conns"`
        ConnMaxLifetime time.Duration `yaml:"conn_max_lifetime"`
        BusyTimeout     time.Duration `yaml:"busy_timeout"`
    } `yaml:"tempolite_db"`
}

// Default optimized configuration
var DefaultDBConfig = DatabaseConfig{
    CoordinationDB: DatabaseConfig{
        MaxOpenConns:    1, // Single writer pattern
        MaxIdleConns:   1,
        ConnMaxLifetime: time.Hour,
        BusyTimeout:     30 * time.Second,
    },
    TempoliteDB: DatabaseConfig{
        MaxOpenConns:    1, // Single writer for SQLite
        MaxIdleConns:   1,
        ConnMaxLifetime: time.Hour,
        BusyTimeout:     30 * time.Second,
    },
}
```

## 🔄 Migration System

### Schema Versioning

```sql
CREATE TABLE schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    checksum TEXT NOT NULL,
    description TEXT,
    execution_time_ms INTEGER,
    success BOOLEAN DEFAULT TRUE
);

-- Initial migration
INSERT INTO schema_migrations (version, checksum, description) VALUES 
    ('1.0.0', 'sha256:abc123', 'Initial schema with workflow coordination');
```

### Migration Scripts

```go
type Migration struct {
    Version     string
    Description string
    UpSQL       []string
    DownSQL     []string
    Checksum    string
}

var Migrations = []Migration{
    {
        Version:     "1.0.0",
        Description: "Initial schema with workflow coordination",
        UpSQL: []string{
            `CREATE TABLE workflow_mappings (...)`,
            `CREATE TABLE agent_assignments (...)`,
            `CREATE TABLE workflow_results (...)`,
            // ... other tables
        },
        DownSQL: []string{
            `DROP TABLE IF EXISTS workflow_mappings`,
            `DROP TABLE IF EXISTS agent_assignments`,
            `DROP TABLE IF EXISTS workflow_results`,
            // ... other tables
        },
        Checksum: "sha256:abc123",
    },
    {
        Version:     "1.1.0",
        Description: "Add performance analytics tables",
        UpSQL: []string{
            `CREATE TABLE workflow_performance (...)`,
            `CREATE TABLE workflow_analytics (...)`,
            `CREATE INDEX idx_workflow_performance_workflow ON workflow_performance(workflow_id)`,
        },
        DownSQL: []string{
            `DROP TABLE IF EXISTS workflow_performance`,
            `DROP TABLE IF EXISTS workflow_analytics`,
        },
        Checksum: "sha256:def456",
    },
}
```

## 🔗 Cross-References

- [System Design](../architecture/01-system-design.md) - Overall system architecture
- [API Design](../api/01-rest-api.md) - API interface design
- [Security Model](../architecture/04-security.md) - Security and access control
- [Deployment Guide](../deployment/01-production.md) - Production deployment
- [Performance Optimization](../implementation/04-performance.md) - Performance tuning

---

**Next**: Read the [API Design](../api/01-rest-api.md) documentation.