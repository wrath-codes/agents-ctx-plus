-- Tempolite workflow engine schema
-- Version: 1.0.0

-- Workflows table
CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY,
    definition JSON NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled', 'paused', 'restored')),
    current_activity TEXT,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_workflows_status ON workflows(status);
CREATE INDEX IF NOT EXISTS idx_workflows_created ON workflows(created_at DESC);

-- Activities table
CREATE TABLE IF NOT EXISTS activities (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    name TEXT NOT NULL,
    params JSON,
    result JSON,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    retry_count INTEGER DEFAULT 0,
    error_message TEXT,
    
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);

CREATE INDEX IF NOT EXISTS idx_activities_workflow ON activities(workflow_id);
CREATE INDEX IF NOT EXISTS idx_activities_status ON activities(status);
CREATE INDEX IF NOT EXISTS idx_activities_name ON activities(name);

-- Sagas table
CREATE TABLE IF NOT EXISTS sagas (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    definition JSON NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed', 'compensating', 'compensated')),
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);

CREATE INDEX IF NOT EXISTS idx_sagas_workflow ON sagas(workflow_id);
CREATE INDEX IF NOT EXISTS idx_sagas_status ON sagas(status);

-- Saga compensation failures table
CREATE TABLE IF NOT EXISTS saga_compensation_failures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    saga_id TEXT NOT NULL,
    step_id TEXT NOT NULL,
    error TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (saga_id) REFERENCES sagas(id)
);

CREATE INDEX IF NOT EXISTS idx_comp_failures_saga ON saga_compensation_failures(saga_id);

-- Signals table
CREATE TABLE IF NOT EXISTS signals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workflow_id TEXT NOT NULL,
    name TEXT NOT NULL,
    data JSON,
    processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL,
    processed_at TIMESTAMP,
    
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);

CREATE INDEX IF NOT EXISTS idx_signals_workflow ON signals(workflow_id);
CREATE INDEX IF NOT EXISTS idx_signals_name ON signals(name);
CREATE INDEX IF NOT EXISTS idx_signals_processed ON signals(processed);

-- Checkpoints table
CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    state JSON NOT NULL,
    created_at TIMESTAMP NOT NULL,
    
    FOREIGN KEY (workflow_id) REFERENCES workflows(id)
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_workflow ON checkpoints(workflow_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_created ON checkpoints(created_at DESC);