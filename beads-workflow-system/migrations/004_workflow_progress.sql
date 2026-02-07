-- Migration: workflow_progress
-- Version: 004
-- Created: Phase 3

CREATE TABLE IF NOT EXISTS workflow_progress (
    workflow_id TEXT PRIMARY KEY,
    current_step INTEGER DEFAULT 0,
    total_steps INTEGER DEFAULT 0,
    progress_percent INTEGER DEFAULT 0,
    status TEXT DEFAULT 'pending',
    current_step_name TEXT,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_workflow_progress_status ON workflow_progress(status);