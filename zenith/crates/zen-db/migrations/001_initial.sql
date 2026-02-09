-- Zenith initial schema
-- Source: 01-turso-data-model.md
-- All statements use IF NOT EXISTS for idempotent re-running.

PRAGMA foreign_keys = ON;

-- ============================================================
-- 1. Project & Session Management
-- ============================================================

CREATE TABLE IF NOT EXISTS project_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS project_dependencies (
    ecosystem TEXT NOT NULL,
    name TEXT NOT NULL,
    version TEXT,
    source TEXT NOT NULL,
    indexed BOOLEAN DEFAULT FALSE,
    indexed_at TEXT,
    PRIMARY KEY (ecosystem, name)
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    summary TEXT
);

CREATE TABLE IF NOT EXISTS session_snapshots (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id),
    open_tasks INTEGER NOT NULL DEFAULT 0,
    in_progress_tasks INTEGER NOT NULL DEFAULT 0,
    pending_hypotheses INTEGER NOT NULL DEFAULT 0,
    unverified_hypotheses INTEGER NOT NULL DEFAULT 0,
    recent_findings INTEGER NOT NULL DEFAULT 0,
    open_research INTEGER NOT NULL DEFAULT 0,
    summary TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 2. Core Knowledge Entities
-- ============================================================

CREATE TABLE IF NOT EXISTS research_items (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS findings (
    id TEXT PRIMARY KEY,
    research_id TEXT REFERENCES research_items(id),
    session_id TEXT REFERENCES sessions(id),
    content TEXT NOT NULL,
    source TEXT,
    confidence TEXT NOT NULL DEFAULT 'medium',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS finding_tags (
    finding_id TEXT NOT NULL REFERENCES findings(id),
    tag TEXT NOT NULL,
    PRIMARY KEY (finding_id, tag)
);

CREATE TABLE IF NOT EXISTS hypotheses (
    id TEXT PRIMARY KEY,
    research_id TEXT REFERENCES research_items(id),
    finding_id TEXT REFERENCES findings(id),
    session_id TEXT REFERENCES sessions(id),
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unverified',
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS insights (
    id TEXT PRIMARY KEY,
    research_id TEXT REFERENCES research_items(id),
    session_id TEXT REFERENCES sessions(id),
    content TEXT NOT NULL,
    confidence TEXT NOT NULL DEFAULT 'medium',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 3. Issues & Work Tracking
-- ============================================================

CREATE TABLE IF NOT EXISTS issues (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL DEFAULT 'task',
    parent_id TEXT REFERENCES issues(id),
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    priority INTEGER NOT NULL DEFAULT 3,
    session_id TEXT REFERENCES sessions(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    research_id TEXT REFERENCES research_items(id),
    issue_id TEXT REFERENCES issues(id),
    session_id TEXT REFERENCES sessions(id),
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS implementation_log (
    id TEXT PRIMARY KEY,
    task_id TEXT REFERENCES tasks(id),
    session_id TEXT REFERENCES sessions(id),
    file_path TEXT NOT NULL,
    start_line INTEGER,
    end_line INTEGER,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 4. Studies
-- ============================================================

CREATE TABLE IF NOT EXISTS studies (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    research_id TEXT REFERENCES research_items(id),
    topic TEXT NOT NULL,
    library TEXT,
    methodology TEXT NOT NULL DEFAULT 'explore',
    status TEXT NOT NULL DEFAULT 'active',
    summary TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 5. Compatibility Tracking
-- ============================================================

CREATE TABLE IF NOT EXISTS compatibility_checks (
    id TEXT PRIMARY KEY,
    package_a TEXT NOT NULL,
    package_b TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unknown',
    conditions TEXT,
    finding_id TEXT REFERENCES findings(id),
    session_id TEXT REFERENCES sessions(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 6. Universal Linking
-- ============================================================

CREATE TABLE IF NOT EXISTS entity_links (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(source_type, source_id, target_type, target_id, relation)
);

-- ============================================================
-- 7. Audit Trail
-- ============================================================

CREATE TABLE IF NOT EXISTS audit_trail (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    detail TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- 8. Full-Text Search (FTS5)
-- ============================================================

CREATE VIRTUAL TABLE IF NOT EXISTS findings_fts USING fts5(
    content, source,
    content='findings',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS hypotheses_fts USING fts5(
    content, reason,
    content='hypotheses',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS insights_fts USING fts5(
    content,
    content='insights',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS research_fts USING fts5(
    title, description,
    content='research_items',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts USING fts5(
    title, description,
    content='tasks',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS issues_fts USING fts5(
    title, description,
    content='issues',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS studies_fts USING fts5(
    topic, summary,
    content='studies',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS audit_fts USING fts5(
    action, detail,
    content='audit_trail',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

-- ============================================================
-- 9. Indexes
-- ============================================================

-- Findings
CREATE INDEX IF NOT EXISTS idx_findings_research ON findings(research_id);
CREATE INDEX IF NOT EXISTS idx_findings_session ON findings(session_id);
CREATE INDEX IF NOT EXISTS idx_findings_confidence ON findings(confidence);
CREATE INDEX IF NOT EXISTS idx_finding_tags_tag ON finding_tags(tag);

-- Hypotheses
CREATE INDEX IF NOT EXISTS idx_hypotheses_research ON hypotheses(research_id);
CREATE INDEX IF NOT EXISTS idx_hypotheses_status ON hypotheses(status);
CREATE INDEX IF NOT EXISTS idx_hypotheses_session ON hypotheses(session_id);

-- Insights
CREATE INDEX IF NOT EXISTS idx_insights_research ON insights(research_id);
CREATE INDEX IF NOT EXISTS idx_insights_session ON insights(session_id);

-- Issues
CREATE INDEX IF NOT EXISTS idx_issues_type ON issues(type);
CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
CREATE INDEX IF NOT EXISTS idx_issues_parent ON issues(parent_id);
CREATE INDEX IF NOT EXISTS idx_issues_session ON issues(session_id);
CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority);

-- Tasks
CREATE INDEX IF NOT EXISTS idx_tasks_research ON tasks(research_id);
CREATE INDEX IF NOT EXISTS idx_tasks_issue ON tasks(issue_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_session ON tasks(session_id);

-- Implementation Log
CREATE INDEX IF NOT EXISTS idx_impl_log_task ON implementation_log(task_id);
CREATE INDEX IF NOT EXISTS idx_impl_log_file ON implementation_log(file_path);
CREATE INDEX IF NOT EXISTS idx_impl_log_session ON implementation_log(session_id);

-- Studies
CREATE INDEX IF NOT EXISTS idx_studies_status ON studies(status);
CREATE INDEX IF NOT EXISTS idx_studies_library ON studies(library);
CREATE INDEX IF NOT EXISTS idx_studies_session ON studies(session_id);
CREATE INDEX IF NOT EXISTS idx_studies_research ON studies(research_id);

-- Compatibility
CREATE INDEX IF NOT EXISTS idx_compat_packages ON compatibility_checks(package_a, package_b);
CREATE INDEX IF NOT EXISTS idx_compat_status ON compatibility_checks(status);

-- Entity Links
CREATE INDEX IF NOT EXISTS idx_entity_links_source ON entity_links(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_entity_links_target ON entity_links(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_entity_links_relation ON entity_links(relation);

-- Audit Trail
CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_trail(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_session ON audit_trail(session_id);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_trail(action);
CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_trail(created_at);

-- Project
CREATE INDEX IF NOT EXISTS idx_project_deps_indexed ON project_dependencies(indexed);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

-- ============================================================
-- 10. FTS5 Sync Triggers
-- ============================================================

-- findings
CREATE TRIGGER IF NOT EXISTS findings_ai AFTER INSERT ON findings BEGIN
    INSERT INTO findings_fts(rowid, content, source)
    VALUES (new.rowid, new.content, new.source);
END;

CREATE TRIGGER IF NOT EXISTS findings_ad AFTER DELETE ON findings BEGIN
    INSERT INTO findings_fts(findings_fts, rowid, content, source)
    VALUES ('delete', old.rowid, old.content, old.source);
END;

CREATE TRIGGER IF NOT EXISTS findings_au AFTER UPDATE ON findings BEGIN
    INSERT INTO findings_fts(findings_fts, rowid, content, source)
    VALUES ('delete', old.rowid, old.content, old.source);
    INSERT INTO findings_fts(rowid, content, source)
    VALUES (new.rowid, new.content, new.source);
END;

-- hypotheses
CREATE TRIGGER IF NOT EXISTS hypotheses_ai AFTER INSERT ON hypotheses BEGIN
    INSERT INTO hypotheses_fts(rowid, content, reason)
    VALUES (new.rowid, new.content, new.reason);
END;

CREATE TRIGGER IF NOT EXISTS hypotheses_ad AFTER DELETE ON hypotheses BEGIN
    INSERT INTO hypotheses_fts(hypotheses_fts, rowid, content, reason)
    VALUES ('delete', old.rowid, old.content, old.reason);
END;

CREATE TRIGGER IF NOT EXISTS hypotheses_au AFTER UPDATE ON hypotheses BEGIN
    INSERT INTO hypotheses_fts(hypotheses_fts, rowid, content, reason)
    VALUES ('delete', old.rowid, old.content, old.reason);
    INSERT INTO hypotheses_fts(rowid, content, reason)
    VALUES (new.rowid, new.content, new.reason);
END;

-- insights
CREATE TRIGGER IF NOT EXISTS insights_ai AFTER INSERT ON insights BEGIN
    INSERT INTO insights_fts(rowid, content)
    VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS insights_ad AFTER DELETE ON insights BEGIN
    INSERT INTO insights_fts(insights_fts, rowid, content)
    VALUES ('delete', old.rowid, old.content);
END;

CREATE TRIGGER IF NOT EXISTS insights_au AFTER UPDATE ON insights BEGIN
    INSERT INTO insights_fts(insights_fts, rowid, content)
    VALUES ('delete', old.rowid, old.content);
    INSERT INTO insights_fts(rowid, content)
    VALUES (new.rowid, new.content);
END;

-- research_items
CREATE TRIGGER IF NOT EXISTS research_ai AFTER INSERT ON research_items BEGIN
    INSERT INTO research_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;

CREATE TRIGGER IF NOT EXISTS research_ad AFTER DELETE ON research_items BEGIN
    INSERT INTO research_fts(research_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, old.description);
END;

CREATE TRIGGER IF NOT EXISTS research_au AFTER UPDATE ON research_items BEGIN
    INSERT INTO research_fts(research_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, old.description);
    INSERT INTO research_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;

-- tasks
CREATE TRIGGER IF NOT EXISTS tasks_ai AFTER INSERT ON tasks BEGIN
    INSERT INTO tasks_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;

CREATE TRIGGER IF NOT EXISTS tasks_ad AFTER DELETE ON tasks BEGIN
    INSERT INTO tasks_fts(tasks_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, old.description);
END;

CREATE TRIGGER IF NOT EXISTS tasks_au AFTER UPDATE ON tasks BEGIN
    INSERT INTO tasks_fts(tasks_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, old.description);
    INSERT INTO tasks_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;

-- issues
CREATE TRIGGER IF NOT EXISTS issues_ai AFTER INSERT ON issues BEGIN
    INSERT INTO issues_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;

CREATE TRIGGER IF NOT EXISTS issues_ad AFTER DELETE ON issues BEGIN
    INSERT INTO issues_fts(issues_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, old.description);
END;

CREATE TRIGGER IF NOT EXISTS issues_au AFTER UPDATE ON issues BEGIN
    INSERT INTO issues_fts(issues_fts, rowid, title, description)
    VALUES ('delete', old.rowid, old.title, old.description);
    INSERT INTO issues_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;

-- studies
CREATE TRIGGER IF NOT EXISTS studies_ai AFTER INSERT ON studies BEGIN
    INSERT INTO studies_fts(rowid, topic, summary)
    VALUES (new.rowid, new.topic, new.summary);
END;

CREATE TRIGGER IF NOT EXISTS studies_ad AFTER DELETE ON studies BEGIN
    INSERT INTO studies_fts(studies_fts, rowid, topic, summary)
    VALUES ('delete', old.rowid, old.topic, old.summary);
END;

CREATE TRIGGER IF NOT EXISTS studies_au AFTER UPDATE ON studies BEGIN
    INSERT INTO studies_fts(studies_fts, rowid, topic, summary)
    VALUES ('delete', old.rowid, old.topic, old.summary);
    INSERT INTO studies_fts(rowid, topic, summary)
    VALUES (new.rowid, new.topic, new.summary);
END;

-- audit_trail (insert only — append-only table)
CREATE TRIGGER IF NOT EXISTS audit_ai AFTER INSERT ON audit_trail BEGIN
    INSERT INTO audit_fts(rowid, action, detail)
    VALUES (new.rowid, new.action, new.detail);
END;
