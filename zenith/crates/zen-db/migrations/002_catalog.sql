-- 002_catalog.sql
-- DuckLake-inspired catalog metadata for cloud index path resolution.

CREATE TABLE IF NOT EXISTS dl_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS dl_snapshot (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    note TEXT
);

CREATE TABLE IF NOT EXISTS dl_data_file (
    id TEXT PRIMARY KEY,
    snapshot_id TEXT NOT NULL,
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    lance_path TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'public',
    org_id TEXT,
    owner_sub TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(snapshot_id) REFERENCES dl_snapshot(id)
);

CREATE INDEX IF NOT EXISTS idx_dl_data_file_pkg
    ON dl_data_file(ecosystem, package, version);

CREATE INDEX IF NOT EXISTS idx_dl_data_file_visibility
    ON dl_data_file(visibility, org_id, owner_sub);

DELETE FROM dl_data_file
WHERE rowid NOT IN (
    SELECT MIN(rowid)
    FROM dl_data_file
    GROUP BY ecosystem, package, version, lance_path
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_dl_data_file_triplet_path
    ON dl_data_file(ecosystem, package, version, lance_path);
