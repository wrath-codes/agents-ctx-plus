-- Migration: tempolite_entity_mapping
-- Adds tempolite_entity_id to workflow_mappings so the int-based
-- WorkflowEntityID assigned by tempolite is persisted alongside
-- our string workflow IDs.

ALTER TABLE workflow_mappings ADD COLUMN tempolite_entity_id INTEGER;

CREATE INDEX IF NOT EXISTS idx_workflow_mappings_entity_id
    ON workflow_mappings(tempolite_entity_id);
