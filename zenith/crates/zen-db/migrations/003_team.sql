-- 003_team.sql
-- Add org_id column to entity tables for team-scoped visibility.
-- NULL = local-only / pre-auth entity. Preserves backward compatibility.

ALTER TABLE sessions ADD COLUMN org_id TEXT;
ALTER TABLE research_items ADD COLUMN org_id TEXT;
ALTER TABLE findings ADD COLUMN org_id TEXT;
ALTER TABLE hypotheses ADD COLUMN org_id TEXT;
ALTER TABLE insights ADD COLUMN org_id TEXT;
ALTER TABLE issues ADD COLUMN org_id TEXT;
ALTER TABLE tasks ADD COLUMN org_id TEXT;
ALTER TABLE studies ADD COLUMN org_id TEXT;
ALTER TABLE implementation_log ADD COLUMN org_id TEXT;
ALTER TABLE compatibility_checks ADD COLUMN org_id TEXT;

-- Index for org_id filtering on frequently queried tables.
CREATE INDEX IF NOT EXISTS idx_sessions_org ON sessions(org_id);
CREATE INDEX IF NOT EXISTS idx_findings_org ON findings(org_id);
CREATE INDEX IF NOT EXISTS idx_tasks_org ON tasks(org_id);
CREATE INDEX IF NOT EXISTS idx_issues_org ON issues(org_id);
CREATE INDEX IF NOT EXISTS idx_research_org ON research_items(org_id);
