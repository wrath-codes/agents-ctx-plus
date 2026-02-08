# Zenith: Studies Feature — Spike Plan

**Version**: 2026-02-08
**Status**: **DONE** — Approach B (hybrid) selected. See [09-studies-workflow.md](./09-studies-workflow.md)
**Purpose**: Design spike to validate the data model approach for a structured learning ("studies") feature
**Spike ID**: 0.11
**Crate**: zen-db
**Blocks**: Design document 09-studies-workflow.md (written after spike concluded)

## Spike Results

**Decision: Approach B (Hybrid) wins.**

All 15 tests pass. The comparison data:

| Metric | Approach A (compose) | Approach B (hybrid) |
|--------|---------------------|---------------------|
| INSERTs (study + 3 assumptions) | 4 | 8 |
| Full-state query (SQL lines) | 8 | 11 |
| Progress query (SQL lines) | 5 | 6 |
| New tables needed | 0 | 1 |
| Filter studies (type-safe?) | No | **Yes** |
| Top-level fields | 4 | **6** |

**Why Approach B wins despite more INSERTs and slightly more complex queries:**
- **Type safety**: `FROM studies WHERE ...` vs `WHERE title LIKE 'Study: %'` — convention-based filtering is fragile
- **Purpose-built fields**: `topic`, `library`, `methodology`, `summary` are first-class columns, not stuffed into `description`
- **Dedicated lifecycle**: `active → concluding → completed | abandoned` separate from research statuses
- **CLI ergonomics**: `znt study create --topic "..." --library tokio` is clearer than `znt research create --title "Study: ..."`
- **The extra INSERTs are entity_links**, which are the same pattern used everywhere else — not additional complexity

**Key finding from the spike**: Approach A's queries are simpler because hypotheses have a direct `research_id` FK, while Approach B routes through `entity_links`. However, Approach B's hypotheses can ALSO use `research_id` (since the study links to a research_item), giving us both the direct FK path AND the entity_links path. Best of both worlds.

---

## Table of Contents

1. [Motivation](#1-motivation)
2. [What Is a Study](#2-what-is-a-study)
3. [Research Summary](#3-research-summary)
4. [The Two Approaches](#4-the-two-approaches)
5. [Spike Scenario](#5-spike-scenario)
6. [Spike Tests](#6-spike-tests)
7. [Evaluation Criteria](#7-evaluation-criteria)
8. [What This Spike Does NOT Test](#8-what-this-spike-does-not-test)
9. [Success Criteria](#9-success-criteria)
10. [Post-Spike Actions](#10-post-spike-actions)

---

## 1. Motivation

Zenith tracks research (questions), findings (facts), hypotheses (assumptions), and insights (conclusions). But there's a gap: **structured learning**.

When a developer asks an LLM "I want to learn how tokio::spawn works," the interaction today is unstructured — a chat conversation that produces no durable artifacts. The knowledge dies with the session.

A "studies" feature would let the LLM:
1. Create a structured learning plan (what to investigate, what assumptions to test)
2. Systematically validate assumptions through code examples and tests
3. Record evidence for each validated/invalidated assumption
4. Produce a structured "what I learned" report that persists across sessions

### The Whitespace

Research across existing tools confirms that **no product today** combines:
- Hypothesis-driven investigation of library/framework behavior
- Executable validation artifacts (tests/examples) as proof of understanding
- First-class assumption tracking with evidence chains
- Integration with actual package docs and source code as ground truth
- Multi-session persistence of learning state

This sits at the intersection of Deep Research (multi-step investigation), TDD (validation through tests), and spaced repetition (structured knowledge management) — but nothing combines them.

---

## 2. What Is a Study

A study is a **structured learning process** where the LLM systematically investigates a topic, validates assumptions through code, and produces documented findings.

### Flow

```
Learning Objective ("How does tokio::spawn work?")
    |
Study Plan (questions to answer, assumptions to test)
    |
Exploration (search docs, read source, form hypotheses)
    |
Validation (write code, run tests, record evidence)
    |
Conclusion (what was confirmed, debunked, remains unknown)
```

### Relationship to Existing Entities

```
Research Item (question)
    -> Study (structured investigation)
        |-- Hypotheses (assumptions to validate)
        |-- Findings (test results + discovered facts)
        +-- Insights (synthesized conclusions)
    -> Tasks (if implementation needed after learning)
```

### How It Differs From Research

| Dimension | Research | Study |
|-----------|----------|-------|
| Goal | Find the answer | Build a mental model |
| Process | Search -> Synthesize -> Report | Hypothesize -> Test -> Refine -> Prove |
| Artifact | Findings + insight | Tests, validated assumptions, structured report |
| Duration | Can be one-shot | Multi-session, iterative |
| Validation | Citation quality | "Can I use this knowledge correctly?" |

### Test Execution Model

The LLM uses its own host tools (bash, file I/O) to write and run test code, then records results in Zenith. Zenith stores the knowledge; the LLM's host environment does execution. This preserves Zenith's "tool, not agent" philosophy.

---

## 3. Research Summary

### Patterns From the Reference Library

**Beads chemistry model** (from `reference/beads/`):
- Wisp (quick exploration) -> Mol (formal study) -> Proto (reusable template)
- A study could follow this promotion path: quick experiment -> formalized study -> study template

**Anthropic's multi-agent research system** (from `reference/llm-context-management/production/02-anthropic-context.md`):
- Orchestrator decomposes query -> sub-agents explore -> condensed findings returned
- Maps to: study plan -> sub-questions explored -> findings recorded
- Key insight: structured note-taking (progress files, JSON checklists) enables cross-session continuity

**Cortex-memory fact extraction** (from `reference/cortex-memory/`):
- Memory categories: Procedural / Factual / Semantic / Episodic
- Maps to study output types: "How to use X" / "X requires Y" / "X relates to Y because..." / "When I tested X, I got Y"
- Enhancement pipeline (extract -> enhance -> store -> retrieve) matches study findings pipeline

**Working Memory Hub** (from `reference/llm-context-management/cognitive/01-working-memory-hub.md`):
- A study IS an episode in the Episodic Buffer — a coherent unit of investigation recallable in future sessions
- The distilled findings become long-term semantic memory in the Hub (Zenith's Turso DB)

### The PRD Precedent

The PRD workflow (`06-prd-workflow.md`) proves complex workflows can be built from existing entities:
- PRD = `issues` (type: epic)
- Tasks = `tasks` linked via `issue_id`
- Relevant files = `findings` tagged `relevant-files`
- Open questions = `hypotheses` (status: unverified)
- No new tables needed

This is the strongest precedent for Approach A. But is it sufficient for studies?

---

## 4. The Two Approaches

### Approach A: Compose From Existing Entities

A study is a `research_item` with conventions (like how PRDs are `issues` with type `epic`).

**Schema changes**: None.

**Conventions**:
- Research title starts with `Study:` (or a tag distinguishes them)
- Research description contains structured study plan markdown
- Hypotheses = assumptions to validate
- Findings tagged `test-result` = test execution results
- Findings tagged `test-code` = the code that was run
- Entity links (`validates`, `debunks`) connect evidence to assumptions
- Insight at conclusion = structured "what I learned" report
- Research status `resolved` = study concluded

**CLI usage**:

```bash
# Create study
znt research create --title "Study: How tokio::spawn works" \
    --description "## Plan\n1. What are the Send bounds?\n2. What happens on panic?"

# Add assumption
znt hypothesis create --content "spawn requires Send + 'static" --research res-xxx

# Record test result
znt finding create --content "Test: spawn non-Send -> E0277" \
    --research res-xxx --tag test-result --confidence high
znt link fnd-xxx hyp-xxx validates

# Validate assumption
znt hypothesis update hyp-xxx --status confirmed --reason "E0277 proves Send required"

# Conclude
znt insight create --content "## Study Conclusions\n### Confirmed\n- spawn requires Send..." \
    --research res-xxx
znt research update res-xxx --status resolved
```

**Full state query** (SQL):

```sql
-- Get study with all related entities
SELECT r.*,
    (SELECT json_group_array(json_object('id', h.id, 'content', h.content, 'status', h.status))
     FROM hypotheses h WHERE h.research_id = r.id) as assumptions,
    (SELECT json_group_array(json_object('id', f.id, 'content', f.content, 'confidence', f.confidence))
     FROM findings f WHERE f.research_id = r.id) as findings,
    (SELECT json_group_array(json_object('id', i.id, 'content', i.content))
     FROM insights i WHERE i.research_id = r.id) as conclusions
FROM research_items r WHERE r.id = ?;
```

### Approach B: Hybrid — One New Table

A `studies` table as the container, with everything else reused.

**Schema changes**: One new table + one new FTS5 + triggers + entity_links support.

```sql
CREATE TABLE studies (
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

CREATE VIRTUAL TABLE studies_fts USING fts5(
    topic, summary,
    content='studies',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

-- Triggers for FTS sync (same pattern as other entities)
```

**ID prefix**: `stu-`

**Entity types**: Add `'study'` to entity_links source/target types.

**Methodology values**: `explore` (open investigation), `test-driven` (hypothesis-first), `compare` (comparing alternatives).

**Status lifecycle**: `active -> concluding -> completed | abandoned`

**CLI usage**:

```bash
# Create study (dedicated command)
znt study create --topic "How tokio::spawn works" --library tokio

# Add assumption (convenience wrapper)
znt study assume stu-xxx --content "spawn requires Send + 'static"

# Record test result (convenience wrapper)
znt study test stu-xxx --assumption hyp-xxx \
    --result validated --evidence "E0277 proves Send required"

# Check progress
znt study get stu-xxx
# -> { assumptions: { total: 3, validated: 2, invalidated: 0, untested: 1 }, ... }

# Conclude
znt study conclude stu-xxx --summary "Tokio's spawn is the fundamental..."
```

**Full state query** (SQL):

```sql
-- Get study with all related entities
SELECT s.*,
    (SELECT json_group_array(json_object('id', h.id, 'content', h.content, 'status', h.status))
     FROM hypotheses h
     JOIN entity_links el ON el.target_type = 'hypothesis' AND el.target_id = h.id
     WHERE el.source_type = 'study' AND el.source_id = s.id) as assumptions,
    (SELECT json_group_array(json_object('id', f.id, 'content', f.content, 'confidence', f.confidence))
     FROM findings f
     JOIN entity_links el ON el.target_type = 'finding' AND el.target_id = f.id
     WHERE el.source_type = 'study' AND el.source_id = s.id) as findings,
    (SELECT json_group_array(json_object('id', i.id, 'content', i.content))
     FROM insights i
     JOIN entity_links el ON el.target_type = 'insight' AND el.target_id = i.id
     WHERE el.source_type = 'study' AND el.source_id = s.id) as conclusions
FROM studies s WHERE s.id = ?;
```

---

## 5. Spike Scenario

Both approaches are tested against the same real-world scenario:

**"Learn how tokio::spawn works"**

Steps:
1. Create the study with a plan (3 questions)
2. Form 3 assumptions:
   - "spawn requires Send + 'static bounds"
   - "spawned tasks can panic without crashing the runtime"
   - "spawn is zero-cost (no allocation at spawn time)"
3. Search indexed docs (simulated — hardcoded results)
4. Run test code (simulated — hardcoded outcomes):
   - Assumption 1: validated (compile error proves it)
   - Assumption 2: validated (panic is caught by runtime)
   - Assumption 3: invalidated (spawn allocates a task harness)
5. Record evidence for each
6. Conclude the study with structured summary
7. Query the full study state
8. Verify study is distinguishable from regular research
9. Verify progress tracking works (3 assumptions, 2 validated, 1 invalidated)

---

## 6. Spike Tests

**File**: `zenith/crates/zen-db/src/spike_studies.rs`

### Part A: Compose-Only Approach (8 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 1 | `spike_a_create_study_as_research` | Create a research_item with "Study:" prefix and structured plan in description. Verify it can be queried and has correct status. |
| 2 | `spike_a_add_assumptions` | Create 3 hypotheses linked to the study research_item via research_id. Verify all start as 'unverified'. |
| 3 | `spike_a_record_test_validated` | Create a finding tagged 'test-result' with evidence. Link it to the hypothesis via entity_links (validates). Update hypothesis to 'confirmed' with reason. |
| 4 | `spike_a_record_test_invalidated` | Create a finding tagged 'test-result' with counter-evidence. Link it to the hypothesis via entity_links (debunks). Update hypothesis to 'debunked' with reason. |
| 5 | `spike_a_conclude_study` | Create an insight with structured conclusion markdown. Update research_item status to 'resolved'. |
| 6 | `spike_a_query_full_state` | Execute the full-state query (research + hypotheses + findings + insights). Verify all entities are returned correctly in a single query. |
| 7 | `spike_a_distinguish_from_research` | Create both a regular research_item and a study research_item. Demonstrate how to filter: by title prefix, by tag, or by convention. Evaluate: is the distinction clear enough? |
| 8 | `spike_a_progress_tracking` | Query: count hypotheses by status for a given research_item. Return: `{ total: 3, confirmed: 2, debunked: 1, unverified: 0 }`. Evaluate: how many queries does this take? |

### Part B: Hybrid Approach (6 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 9 | `spike_b_create_study_table` | Create the studies table + FTS5 + triggers. Insert a study row. Verify it persists and FTS works. |
| 10 | `spike_b_add_assumptions_via_links` | Create hypotheses. Link to study via entity_links. Verify the link query works. |
| 11 | `spike_b_record_and_validate` | Create findings linked to study + hypothesis. Update hypothesis status. Verify entity_link chain: study -> hypothesis, finding -> hypothesis (validates). |
| 12 | `spike_b_conclude_study` | Update study status to 'completed', set summary. Create insight linked to study. Verify the full lifecycle. |
| 13 | `spike_b_query_full_state` | Execute the dedicated study query (study + joins). Verify all entities returned. Compare SQL complexity to Approach A. |
| 14 | `spike_b_progress_tracking` | Query assumption progress directly from the study. Return: `{ total: 3, validated: 2, invalidated: 1, untested: 0 }`. Compare: is this simpler than Approach A's version? |

### Part C: Comparison (1 test)

| # | Test | What It Validates |
|---|------|-------------------|
| 15 | `spike_compare_approaches` | Run both approaches side by side. Log: lines of SQL for full-state query (A vs B), number of INSERT statements to create a study (A vs B), number of queries to get progress (A vs B), whether Approach A can filter studies from research. Print a comparison table in test output. |

**Total: 15 tests**

---

## 7. Evaluation Criteria

| Criterion | Weight | How We Measure |
|-----------|--------|---------------|
| Query ergonomics | High | Lines of SQL for "get me the full study state" |
| CLI naturalness | High | Does `znt study create` feel better than `znt research create --title "Study: ..."` for the LLM? |
| Data clarity | Medium | Can we distinguish studies from regular research at query time? |
| Schema cost | Medium | 1 new table + FTS + triggers vs 0 changes |
| Progress tracking | High | How easy is "3 assumptions, 2 validated, 1 untested"? |
| Reporting | Medium | How easy is generating a structured "what I learned" summary? |
| Multi-session | Low | Both should work (research_items and studies both persist) |

---

## 8. What This Spike Does NOT Test

- Actual LLM interaction or CLI commands (spike is pure SQL + Rust)
- fastembed / DuckDB / search integration
- Real package documentation searching
- Multi-session behavior (simulated with hardcoded session IDs)
- Performance at scale (spike uses 3 assumptions, not 300)
- Test code execution (Zenith doesn't execute; the LLM's host does)

---

## 9. Success Criteria

- Both approaches compile and all 15 tests pass
- Concrete SQL queries documented for both approaches
- Clear recommendation made with evidence from test results
- Recommendation documented in spike module doc comments (following spike 0.2-0.9 pattern)
- Decision captured: "Use Approach A because..." or "Use Approach B because..."

---

## 10. Post-Spike Actions

### If Approach A Wins (Compose Only)

1. Write `08-studies-workflow.md` following the PRD workflow pattern:
   - Document conventions (title prefix, tags, description format)
   - Document CLI usage patterns
   - Document the study lifecycle using existing entity statuses
   - No schema changes to `01-turso-data-model.md`
2. Add `znt study` as convenience aliases in `04-cli-api-design.md` that map to existing commands
3. Update `07-implementation-plan.md`: studies feature ships with Phase 5 (MVP) — no extra work

### If Approach B Wins (Hybrid)

1. Write `08-studies-workflow.md` with full study lifecycle documentation
2. Update `01-turso-data-model.md`: add `studies` table, `studies_fts`, triggers, indexes
3. Update `04-cli-api-design.md`: add `znt study` command tree
4. Update `03-architecture-overview.md`: add `stu-` prefix, study entity type
5. Update `05-crate-designs.md`: add `StudyRepo` to zen-db
6. Update `07-implementation-plan.md`:
   - Add `StudyRepo` to Phase 2 tasks
   - Add study CLI commands to Phase 5 tasks
   - Update entity count references
7. Update `INDEX.md` quick reference: add study to ENTITIES and PREFIXES

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- PRD workflow (composition precedent): [06-prd-workflow.md](./06-prd-workflow.md)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md)
- Reference: Anthropic research system — `reference/llm-context-management/production/02-anthropic-context.md`
- Reference: Cortex memory pipeline — `reference/cortex-memory/`
- Reference: Beads chemistry model — `reference/beads/workflows/chemistry-metaphor.md`
- Reference: Working Memory Hub — `reference/llm-context-management/cognitive/01-working-memory-hub.md`
