-- Beads issue tracking schema
-- Version: 1.0.0

-- Issues table
CREATE TABLE IF NOT EXISTS issues (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    type TEXT,
    priority INTEGER DEFAULT 2 CHECK (priority BETWEEN 0 AND 3),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'in_progress', 'blocked', 'closed', 'cancelled')),
    assignee TEXT,
    labels JSON,
    parent_id TEXT,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    closed_at TIMESTAMP,
    
    FOREIGN KEY (parent_id) REFERENCES issues(id)
);

CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority DESC);
CREATE INDEX IF NOT EXISTS idx_issues_assignee ON issues(assignee);
CREATE INDEX IF NOT EXISTS idx_issues_parent ON issues(parent_id);
CREATE INDEX IF NOT EXISTS idx_issues_created ON issues(created_at DESC);

-- Dependencies table
CREATE TABLE IF NOT EXISTS dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id TEXT NOT NULL,
    child_id TEXT NOT NULL,
    dep_type TEXT DEFAULT 'blocks' CHECK (dep_type IN ('blocks', 'depends_on', 'relates_to', 'duplicates')),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (parent_id) REFERENCES issues(id),
    FOREIGN KEY (child_id) REFERENCES issues(id),
    UNIQUE(parent_id, child_id)
);

CREATE INDEX IF NOT EXISTS idx_deps_parent ON dependencies(parent_id);
CREATE INDEX IF NOT EXISTS idx_deps_child ON dependencies(child_id);

-- Comments table
CREATE TABLE IF NOT EXISTS comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL,
    author TEXT,
    content TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP,
    
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_comments_issue ON comments(issue_id);
CREATE INDEX IF NOT EXISTS idx_comments_created ON comments(created_at DESC);

-- Formulas table (workflow templates)
CREATE TABLE IF NOT EXISTS formulas (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    definition JSON NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_formulas_active ON formulas(is_active);

-- Molecules table (instantiated workflows)
CREATE TABLE IF NOT EXISTS molecules (
    id TEXT PRIMARY KEY,
    formula_name TEXT NOT NULL,
    variables JSON,
    status TEXT DEFAULT 'active' CHECK (status IN ('active', 'completed', 'failed', 'cancelled')),
    created_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    
    FOREIGN KEY (formula_name) REFERENCES formulas(name)
);

CREATE INDEX IF NOT EXISTS idx_molecules_formula ON molecules(formula_name);
CREATE INDEX IF NOT EXISTS idx_molecules_status ON molecules(status);