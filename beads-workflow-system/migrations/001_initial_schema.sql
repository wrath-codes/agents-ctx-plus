-- Initial schema migration for beads-workflow-system
-- Version: 1.0.0

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- Schema migrations table
CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    checksum TEXT NOT NULL,
    description TEXT,
    execution_time_ms INTEGER,
    success BOOLEAN DEFAULT TRUE
);

-- Workflow mappings: Bridge between beads issues and tempolite workflows
CREATE TABLE IF NOT EXISTS workflow_mappings (
    tempolite_workflow_id TEXT PRIMARY KEY,
    beads_issue_id TEXT,
    workflow_type TEXT NOT NULL CHECK (workflow_type IN ('research', 'poc', 'documentation', 'validation')),
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'completed', 'failed', 'paused', 'cancelled')),
    priority INTEGER DEFAULT 2 CHECK (priority BETWEEN 0 AND 3),
    metadata JSON,
    parent_workflow_id TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    
    FOREIGN KEY (parent_workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes for workflow_mappings
CREATE INDEX IF NOT EXISTS idx_workflow_mappings_tempolite_id ON workflow_mappings(tempolite_workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_mappings_type_status ON workflow_mappings(workflow_type, status);
CREATE INDEX IF NOT EXISTS idx_workflow_mappings_status_priority ON workflow_mappings(status, priority);
CREATE INDEX IF NOT EXISTS idx_workflow_mappings_parent ON workflow_mappings(parent_workflow_id);

-- Agent assignments: Track agent assignments to workflows
CREATE TABLE IF NOT EXISTS agent_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL CHECK (agent_type IN ('research', 'poc', 'documentation', 'validation', 'supervisor')),
    agent_id TEXT NOT NULL,
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

-- Indexes for agent_assignments
CREATE INDEX IF NOT EXISTS idx_agent_assignments_agent_status ON agent_assignments(agent_id, status);
CREATE INDEX IF NOT EXISTS idx_agent_assignments_workflow ON agent_assignments(workflow_id, step_number);
CREATE INDEX IF NOT EXISTS idx_agent_assignments_agent_type ON agent_assignments(agent_type, status);

-- Workflow results: Store execution results
CREATE TABLE IF NOT EXISTS workflow_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    result_type TEXT NOT NULL CHECK (result_type IN ('findings', 'poc_results', 'documentation', 'validation', 'performance')),
    result_data JSON NOT NULL,
    confidence_score REAL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    execution_time_ms INTEGER,
    artifacts JSON,
    metadata JSON,
    quality_score REAL CHECK (quality_score >= 0.0 AND quality_score <= 10.0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes for workflow_results
CREATE INDEX IF NOT EXISTS idx_workflow_results_workflow ON workflow_results(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_results_agent_type ON workflow_results(agent_type, result_type);
CREATE INDEX IF NOT EXISTS idx_workflow_results_quality ON workflow_results(quality_score DESC);
CREATE INDEX IF NOT EXISTS idx_workflow_results_created ON workflow_results(created_at DESC);

-- Research findings: Detailed storage for research results
CREATE TABLE IF NOT EXISTS research_findings (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    library_name TEXT NOT NULL,
    library_version TEXT,
    documentation_url TEXT,
    findings JSON NOT NULL,
    confidence_score REAL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    relevance_score REAL CHECK (relevance_score >= 0.0 AND relevance_score <= 1.0),
    analysis_method TEXT,
    file_paths JSON,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes for research_findings
CREATE INDEX IF NOT EXISTS idx_research_findings_workflow ON research_findings(workflow_id);
CREATE INDEX IF NOT EXISTS idx_research_findings_library ON research_findings(library_name, library_version);
CREATE INDEX IF NOT EXISTS idx_research_findings_confidence ON research_findings(confidence_score DESC);

-- POC results: Detailed storage for proof-of-concept results
CREATE TABLE IF NOT EXISTS poc_results (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    implementation_type TEXT NOT NULL CHECK (implementation_type IN ('function', 'module', 'service', 'full_application')),
    language TEXT NOT NULL,
    framework TEXT,
    build_success BOOLEAN DEFAULT FALSE,
    test_success BOOLEAN DEFAULT FALSE,
    performance_metrics JSON,
    test_results JSON,
    artifacts JSON,
    benchmarks JSON,
    error_message TEXT,
    execution_time_ms INTEGER,
    complexity_score REAL CHECK (complexity_score >= 0.0 AND complexity_score <= 10.0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflow_mappings(tempolite_workflow_id)
);

-- Indexes for poc_results
CREATE INDEX IF NOT EXISTS idx_poc_results_workflow ON poc_results(workflow_id);
CREATE INDEX IF NOT EXISTS idx_poc_results_success ON poc_results(build_success, test_success);
CREATE INDEX IF NOT EXISTS idx_poc_results_performance ON poc_results(execution_time_ms DESC);

-- Workflow performance: Performance metrics
CREATE TABLE IF NOT EXISTS workflow_performance (
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

-- Indexes for workflow_performance
CREATE INDEX IF NOT EXISTS idx_workflow_performance_workflow ON workflow_performance(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_performance_duration ON workflow_performance(duration_ms DESC);
CREATE INDEX IF NOT EXISTS idx_workflow_performance_success ON workflow_performance(success, duration_ms);

-- Workflow analytics: Aggregated analytics
CREATE TABLE IF NOT EXISTS workflow_analytics (
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

-- Indexes for workflow_analytics
CREATE INDEX IF NOT EXISTS idx_workflow_analytics_date ON workflow_analytics(date DESC);
CREATE INDEX IF NOT EXISTS idx_workflow_analytics_type ON workflow_analytics(workflow_type, agent_type);

-- Workflow templates: Reusable templates
CREATE TABLE IF NOT EXISTS workflow_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    workflow_type TEXT NOT NULL,
    template_def JSON NOT NULL,
    agent_sequence JSON NOT NULL,
    variables JSON,
    version TEXT DEFAULT '1.0',
    is_active BOOLEAN DEFAULT TRUE,
    usage_count INTEGER DEFAULT 0,
    success_rate REAL DEFAULT 0.0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for workflow_templates
CREATE INDEX IF NOT EXISTS idx_workflow_templates_type ON workflow_templates(workflow_type);
CREATE INDEX IF NOT EXISTS idx_workflow_templates_active ON workflow_templates(is_active, usage_count DESC);

-- Agent configurations: Agent-specific configs
CREATE TABLE IF NOT EXISTS agent_configurations (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    configuration JSON NOT NULL,
    capabilities JSON,
    max_workload INTEGER DEFAULT 5,
    current_workload INTEGER DEFAULT 0,
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'busy', 'error')),
    last_heartbeat TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    performance_metrics JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE(agent_type, agent_id)
);

-- Indexes for agent_configurations
CREATE INDEX IF NOT EXISTS idx_agent_configurations_type_status ON agent_configurations(agent_type, status);
CREATE INDEX IF NOT EXISTS idx_agent_configurations_workload ON agent_configurations(current_workload, max_workload);