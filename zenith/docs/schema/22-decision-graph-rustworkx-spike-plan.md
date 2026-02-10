# Spike 0.22: Decision Traces + Context Graph Spike Plan

**Version**: 2026-02-10 (rev 2 — advanced path with full schema)
**Status**: **DONE** — 54/54 tests pass (Phase A: 37/37 zen-db, Phase B: 10/10 zen-search + 7/7 zen-db integration)  
**Purpose**: Validate whether Zenith can serve as a **system of record for decisions** — not just a filing cabinet of what changed, but a queryable graph of *why* changes were made, what evidence was evaluated, what options were considered, and what precedent informed each choice — while preserving all Phase 2 invariants.

---

## 1. The Insight: What Zenith Is Missing

Zenith already captures *what changed* (JSONL trail + audit trail) and *how things relate* (entity_links). What it does **not** capture is *why a change was justified*.

Consider what happens today when an LLM confirms a hypothesis:

```
JSONL trail says:  {"op":"update","entity":"hypothesis","id":"hyp-001","data":{"status":"confirmed"}}
Audit trail says:  {"action":"status_changed","entity_type":"hypothesis","detail":{"from":"unverified","to":"confirmed"}}
```

Both record the **fact** of the change. Neither records:
- What evidence the LLM evaluated before confirming
- What alternative interpretations it considered
- Whether this follows precedent from a similar decision in a past session
- Whether it violated or upheld a project guideline

This is the "decision trace gap" described in Foundation Capital's [Context Graphs thesis](https://foundationcapital.com/context-graphs-ais-trillion-dollar-opportunity/): enterprise systems store final state but discard the reasoning that justified it. The same gap exists in Zenith, and it matters more here because:

1. **The brain is swappable.** Any LLM, any session. Without durable decision records, each new session starts from scratch — no institutional memory of *why* things were decided.
2. **`whats-next` is currently blind to reasoning.** It reports counts (open tasks, pending hypotheses) but cannot surface "here's a similar decision from session X with this evidence chain."
3. **Studies already capture half the picture.** The studies workflow (`assume → test → validate → conclude`) creates evidence chains, but the *verdict* (confirm/debunk) has no structured justification attached.

### Source Concepts

This spike draws from three reference materials:

- **Context Graphs** (`reference/llm-context-management/context-graphs.md`): Decision traces as first-class data; compounding precedent flywheel; "event sourcing for business judgment."
- **G-Memory** (`reference/llm-context-management/strategies/04-advanced-strategies.md`): Three-tier graph hierarchy (Insight → Query/Pattern → Interaction) with bi-directional traversal.
- **KGGen** (`reference/llm-context-management/kggen_paper.md`): Entity clustering and relation extraction from unstructured text — relevant for future automated decision trace extraction.

---

## 2. What "Decision" Means in Zenith

Not every LLM micro-choice is a decision worth recording. A decision in Zenith is **a moment where the orchestrator (LLM + human) commits to one of multiple plausible alternatives, resulting in a visible state change or durable claim.**

Five categories, ordered by precedent value:

### 2.1 Verdict decisions (truth maintenance)
- Hypothesis: `unverified → confirmed / debunked / inconclusive`
- Study conclusion: "these are the validated takeaways"
- The LLM asserts: "this is now believed true enough to act on."

### 2.2 Architecture / dependency decisions
- "Use libsql not turso crate because FTS stability"
- "Choose Approach B from spike 0.11"
- "Adopt ast-grep instead of raw tree-sitter"
- Precedent-rich; recur across projects; currently buried in session summaries.

### 2.3 Planning / priority decisions
- Task ordering ("do X before Y, because Y depends on X's output")
- Scope boundaries in PRDs ("explicit non-goal: no server mode in MVP")
- Future sessions need "why did we choose this ordering/scope?"

### 2.4 Exception / override decisions
- "We violated our own guideline because..."
- "Accepted a hack/TODO debt intentionally; risk acknowledged"
- Without these, future LLMs will "fix" things that were deliberate.

### 2.5 Completion / acceptance decisions
- Task marked done (with what verification evidence?)
- PRD declared complete
- Establishes "what counts as finished."

---

## 3. Design Principle: Decision Traces as Entity Hub Nodes

A decision trace is a **hub node** in Zenith's entity graph, connecting evidence, policy, options, outcomes, and precedent:

```
Evidence entities ──┐
                    │    ┌──► Option A (chosen) ──► per-option evidence
Policy/guideline ───┤──► Decision ──► Option B (rejected) ──► per-option evidence
                    │    └──► Option C (rejected)
Precedent traces ───┘         │
                              ├──► Outcome entity A (validates hypothesis)
                              └──► Outcome entity B (creates task)
```

This maps to G-Memory's three tiers, grounded in Zenith's actual entity hierarchy:

| G-Memory tier | Zenith equivalent | Role |
|---|---|---|
| Insight Graph (high-level) | Insights, policies, guidelines | Rules of thumb; "what we believe" |
| Query Graph (mid-level patterns) | Research, Studies, PRDs, **Decision traces** | Evidence chains, structured investigations, justified choices |
| Interaction Graph (fine-grained) | JSONL trail, audit trail, `znt log` | Raw events; "what happened when" |

**Decision traces sit between the mid and high levels**: they turn a pile of evidence + a policy into a justified outcome, plus precedent pointers.

---

## 4. Hard Constraints (Must Hold)

All work must preserve Phase 2 rules from `20-phase2-storage-layer-plan.md`:

1. **Mutation ordering**: `BEGIN → SQL → audit → trail → COMMIT`
2. **Replay invariant**: DB state rebuildable from trail operations
3. **Versioning invariant**: trail envelope `v` + version dispatch on replay
4. **Validation invariant**: trail write is warn-only; strict enforcement on replay
5. **Null/params invariant**: fixed INSERT/SELECT use `params!` + `Option<T>`; dynamic UPDATE uses `Vec<libsql::Value>`
6. **No empty-string nullable hack**
7. **Determinism invariant**: graph outputs stable across repeated runs
8. **Budget invariant**: graph expansion bounded

---

## 5. Research Questions and Hypotheses

### RQ1: Does a first-class `decisions` schema produce reliable structured queries and FTS retrieval?

**H1**: A `decisions` table with indexed columns for `category`, `subject_type`, `confidence`, and `exception_kind` — plus a denormalized `search_text` column for FTS — produces reliable precedent queries without JSON parsing.

### RQ2: Does precedent retrieval produce useful results for `whats-next`?

**H2**: When a new session starts with open tasks or unresolved hypotheses, searching for prior decision traces with similar subjects/evidence produces relevant precedent that improves the LLM's next action. Measured by precision@5 over a gold set.

### RQ3: Do `rustworkx-core` graph algorithms add value over SQL-based queries?

**H3**: Graph-based operations (shortest path for explainability, centrality for influence ranking, toposort for task ordering, cycle detection) materially improve output quality over flat SQL queries alone.

### RQ4: Are graph outputs deterministic and bounded at realistic scale?

**H4**: With stable insertion order + tie-break policy + hard budget caps, graph outputs are byte-for-byte stable across runs.

### RQ5: Does per-option evidence structure add retrievable value over flat evidence linking?

**H5**: Queries like "show decisions where the rejected option had strong evidence" and "find precedents where the same alternative was evaluated" produce useful results that flat decision→evidence links cannot express.

---

## 6. Two-Phase Spike Design

### Phase A: Decision Trace Model + Precedent Search

Validate RQ1, RQ2, RQ5 using the first-class `decisions` schema with `decision_options`, `decision_option_evidence`, `decision_outcomes`, entity_links, FTS, trail, and replay.

**Phase A success gate**: data can be stored + replayed + baseline precedent retrieval returns relevant results at acceptable precision@5.

### Phase B: Graph Analytics Engine

Validate RQ3, RQ4 using `rustworkx-core` for algorithms over the subgraph constructed from Phase A's entity_links.

Phase B tests **incremental algorithm value** (explainability, centrality, DAG ops). It does not rescue Phase A fundamentals.

---

## 7. Data Model

### 7.1 `decisions` (canonical record)

```sql
CREATE TABLE decisions (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id),

    -- Context Graph essentials
    category TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    question TEXT NOT NULL,
    because TEXT NOT NULL,
    outcome_summary TEXT,

    -- Policy + exception
    policy_type TEXT,
    policy_id TEXT,
    exception_kind TEXT,
    exception_reason TEXT,

    -- Approval
    approver TEXT,

    -- Scoring
    confidence TEXT NOT NULL,

    -- FTS source (denormalized, built at write time)
    search_text TEXT NOT NULL,

    -- Forward-compat
    metadata_json TEXT,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX decisions_subject_idx ON decisions(subject_type, subject_id);
CREATE INDEX decisions_session_idx ON decisions(session_id);
CREATE INDEX decisions_category_idx ON decisions(category);
CREATE INDEX decisions_created_idx ON decisions(created_at);
CREATE INDEX decisions_confidence_idx ON decisions(confidence);
CREATE INDEX decisions_exception_idx ON decisions(exception_kind);
```

**Column mapping to Foundation Capital fields**:

| Context Graph field | Column | Notes |
|---|---|---|
| Inputs gathered | Via `entity_links` + `decision_option_evidence` | Not a single column; graph edges |
| Policy evaluated | `policy_type`, `policy_id` | Links to Insight or Finding |
| Exception invoked | `exception_kind`, `exception_reason` | Queryable without JSON parsing |
| Approver | `approver` | `"human:<name>"`, `"llm"`, `"pair"` |
| Outcome | Via `decision_outcomes` + `entity_links` | What entities were mutated |
| Precedent | Via `entity_links` (`FollowsPrecedent`) | Decision → Decision edges |

### 7.2 `decision_options` (options considered)

```sql
CREATE TABLE decision_options (
    id TEXT PRIMARY KEY,
    decision_id TEXT NOT NULL REFERENCES decisions(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    summary TEXT,
    is_chosen INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX decision_options_one_chosen
    ON decision_options(decision_id) WHERE is_chosen = 1;
CREATE INDEX decision_options_decision_idx ON decision_options(decision_id);
CREATE INDEX decision_options_label_idx ON decision_options(label);
```

**Why per-option structure matters**: enables queries that the Finding-based approach cannot express:
- "Show decisions where option X was considered but rejected"
- "Show evidence that supported the rejected option"
- "Find precedents where the same alternative was evaluated"

### 7.3 `decision_option_evidence` (per-option inputs)

```sql
CREATE TABLE decision_option_evidence (
    option_id TEXT NOT NULL REFERENCES decision_options(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    PRIMARY KEY(option_id, entity_type, entity_id)
);

CREATE INDEX decision_option_evidence_entity_idx
    ON decision_option_evidence(entity_type, entity_id);
```

### 7.4 `decision_outcomes` (what the decision changed)

```sql
CREATE TABLE decision_outcomes (
    decision_id TEXT NOT NULL REFERENCES decisions(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    PRIMARY KEY(decision_id, entity_type, entity_id, relation)
);

CREATE INDEX decision_outcomes_entity_idx
    ON decision_outcomes(entity_type, entity_id);
```

**Why a dedicated outcomes table**: a decision can simultaneously confirm a hypothesis *and* create follow-up tasks *and* supersede a prior decision. `decision_outcomes` normalizes these multi-target results with typed relations, and keeps `entity_links` available for graph traversal of the same edges.

### 7.5 `decisions_fts` (full-text search)

```sql
CREATE VIRTUAL TABLE decisions_fts USING fts5(
    search_text,
    content='decisions',
    content_rowid='rowid'
);
```

With the standard trigger pattern (INSERT/UPDATE/DELETE) already established by existing FTS-backed tables.

**`search_text` construction** (built deterministically in Rust at write time):

```
{question}
chosen: {chosen_option_label}
because: {because}
options: {option_1_label}: {option_1_summary}, {option_2_label}: {option_2_summary}, ...
exception: {exception_kind} {exception_reason}
outcome: {outcome_summary}
```

This keeps FTS clean (no JSON syntax noise) while making all key fields searchable.

### 7.6 Graph Edges via `entity_links`

Decisions participate in the universal entity graph through `entity_links`. These mirror `decision_outcomes` and `decision_option_evidence` for graph traversal, plus add precedent/policy edges:

| Edge | Source | Target | Relation |
|---|---|---|---|
| Evidence input | Decision | Finding/Research/Study | `DerivedFrom` |
| Verdict outcome | Decision | Hypothesis | `Validates` / `Debunks` |
| Task outcome | Decision | Task | `Implements` / `Blocks` |
| Supersedes prior | Decision | Decision | `Supersedes` |
| Follows precedent | Decision | Decision | `FollowsPrecedent` (new) |
| Overrides policy | Decision | Insight/Finding | `OverridesPolicy` (new) |
| General association | Decision | any entity | `RelatesTo` |

---

## 8. Entity Type and Relation Extensions

### 8.1 New EntityType

Add `Decision` to the `EntityType` enum:

```rust
pub enum EntityType {
    // ... existing variants ...
    Decision,
}
```

`DecisionOption` is **not** promoted to EntityType. Options are sub-rows of a decision, not globally addressable entities. They are queryable via joins, not via `entity_links`.

**Trigger for future promotion**: if options need to link to entities beyond evidence (e.g., tasks implementing a specific option, or follow-up decisions refining a specific rejected option) via `entity_links`.

### 8.2 New Relations

Add two relations to the `Relation` enum:

```rust
pub enum Relation {
    // ... existing variants ...
    FollowsPrecedent,
    OverridesPolicy,
}
```

| Relation | Semantics | Distinction from existing |
|---|---|---|
| `FollowsPrecedent` | "This decision was informed by and follows the approach of a prior decision" | `RelatesTo` is too weak — loses "precedent" semantics. `DerivedFrom` implies content transformation, not precedent-following. |
| `OverridesPolicy` | "This decision explicitly violates or overrides a policy/guideline" | `Supersedes` implies replacement of the target; `OverridesPolicy` means the policy still stands but was excepted in this case. |

### 8.3 `Supersedes` vs `OverridesPolicy` Distinction

| Concept | Relation | Effect on target |
|---|---|---|
| "We changed our mind; this replaces the old decision" | `Supersedes` | Old decision is no longer current |
| "We violated policy X in this specific case" | `OverridesPolicy` | Policy still stands; exception is case-specific |

This distinction matters for precedent retrieval: a superseded decision is deprecated; an overridden policy is still active and may apply to future cases.

---

## 9. Trail Operations

### 9.1 Composite `decision_create` Operation

A decision trace spans 4 tables + entity_links. To prevent partial-trail corruption from multi-line appends, `decision_create` is a **single composite trail operation** (one JSONL line) that the replayer expands into multiple inserts within a single transaction.

```json
{
    "v": 1,
    "ts": "2026-02-10T20:00:00Z",
    "ses": "ses-abc123",
    "op": "decision_create",
    "entity": "decision",
    "id": "dec-a1b2c3d4",
    "data": {
        "decision": {
            "id": "dec-a1b2c3d4",
            "session_id": "ses-abc123",
            "category": "verdict",
            "subject_type": "hypothesis",
            "subject_id": "hyp-001",
            "question": "Should we confirm that tokio::spawn requires Send + 'static?",
            "because": "3 independent code tests prove Send + 'static is required",
            "outcome_summary": "hypothesis confirmed",
            "policy_type": null,
            "policy_id": null,
            "exception_kind": null,
            "exception_reason": null,
            "approver": "llm",
            "confidence": "high"
        },
        "options": [
            {
                "id": "opt-001",
                "label": "confirm",
                "summary": "E0277 error proves Send bound at compile time",
                "is_chosen": true,
                "sort_order": 0,
                "evidence": [
                    { "type": "finding", "id": "fnd-abc" },
                    { "type": "finding", "id": "fnd-def" }
                ]
            },
            {
                "id": "opt-002",
                "label": "inconclusive",
                "summary": "Could be a special case of the test setup",
                "is_chosen": false,
                "sort_order": 1,
                "evidence": [
                    { "type": "finding", "id": "fnd-ghi" }
                ]
            }
        ],
        "outcomes": [
            { "entity_type": "hypothesis", "entity_id": "hyp-001", "relation": "validates" }
        ],
        "links": [
            { "source_type": "decision", "source_id": "dec-a1b2c3d4", "target_type": "finding", "target_id": "fnd-abc", "relation": "derived_from" },
            { "source_type": "decision", "source_id": "dec-a1b2c3d4", "target_type": "finding", "target_id": "fnd-def", "relation": "derived_from" },
            { "source_type": "decision", "source_id": "dec-a1b2c3d4", "target_type": "hypothesis", "target_id": "hyp-001", "relation": "validates" }
        ]
    }
}
```

### 9.2 Replay Behavior

The replayer opens a single DB transaction and performs:

1. INSERT into `decisions` (with computed `search_text`)
2. INSERT into `decision_options` (one per option)
3. INSERT into `decision_option_evidence` (one per option-evidence pair)
4. INSERT into `decision_outcomes` (one per outcome)
5. INSERT into `entity_links` (one per link)
6. INSERT into `audit_trail`

If any step fails → transaction rolls back → no partial state.

Strict replay validates the `data` payload against the decision schema (via zen-schema). Warn-only on write; strict on replay.

### 9.3 Other Trail Operations

- `decision_update` — patch `because`, `confidence`, `outcome_summary`, `approver`, `exception_kind`, `exception_reason`. Uses `Vec<libsql::Value>` for dynamic SET clauses. Single JSONL line.
- `decision_link_precedent` — adds a `FollowsPrecedent` or `OverridesPolicy` entity_link after creation. Single JSONL line.

### 9.4 Decisions Are Not Edited, They Are Superseded

A decision trace records what was believed at decision time. If the decision is later reversed:

1. Create a **new** decision trace
2. Link new → old via `Supersedes`
3. The old trace remains as historical record

This preserves the "case law" property: you can always see the full history of reasoning, including what was later overturned.

---

## 10. Precedent Search

### 10.1 Query Design

Given a subject entity (e.g., a hypothesis about to be evaluated), find relevant prior decision traces. The query uses SQL + FTS + entity_links — no graph algorithms required for baseline retrieval.

**Parameters**:
- `?1` = `subject_type` (e.g., "hypothesis")
- `?2` = `subject_id` (e.g., "hyp-003")
- `?3` = FTS query string (built from subject title/content keywords)
- `?4` = current timestamp (RFC3339)
- `?5` = limit

```sql
WITH subject_findings AS (
    SELECT el.source_id AS finding_id
    FROM entity_links el
    WHERE el.source_type = 'finding'
      AND el.target_type = ?1
      AND el.target_id = ?2
      AND el.relation IN ('relates_to', 'validates', 'debunks', 'derived_from')
),
fts_hits AS (
    SELECT rowid AS d_rowid,
           bm25(decisions_fts) AS fts_rank
    FROM decisions_fts
    WHERE decisions_fts MATCH ?3
),
shared_evidence AS (
    SELECT el.source_id AS decision_id,
           COUNT(*) AS shared_count
    FROM entity_links el
    JOIN subject_findings sf ON sf.finding_id = el.target_id
    WHERE el.source_type = 'decision'
      AND el.target_type = 'finding'
      AND el.relation = 'derived_from'
    GROUP BY el.source_id
)
SELECT
    d.id,
    d.session_id,
    d.category,
    d.subject_type,
    d.subject_id,
    d.question,
    d.because,
    d.confidence,
    d.exception_kind,
    d.created_at,
    COALESCE(se.shared_count, 0) AS shared_evidence,
    COALESCE(f.fts_rank, 9999.0) AS fts_rank,
    (
        COALESCE(se.shared_count, 0) * 10.0
        + (CASE d.confidence WHEN 'high' THEN 3 WHEN 'medium' THEN 2 ELSE 1 END) * 2.0
        - (julianday(?4) - julianday(d.created_at)) * 0.2
        - COALESCE(f.fts_rank, 50.0)
    ) AS score
FROM decisions d
LEFT JOIN shared_evidence se ON se.decision_id = d.id
LEFT JOIN fts_hits f ON f.d_rowid = d.rowid
WHERE f.d_rowid IS NOT NULL
   OR se.shared_count IS NOT NULL
ORDER BY score DESC, d.id ASC
LIMIT ?5;
```

**Join count**: 2 CTEs + 2 LEFT JOINs + 1 JOIN. With proper indexes on `entity_links` and `decisions(created_at)`, this is practical at CLI scale. No graph algorithms required.

### 10.2 Evaluation Methodology

To objectively measure precedent retrieval quality:

1. Define 10–20 queries from fixture data with a **gold set** of relevant decision IDs per query
2. For each query, compute **precision@5** and **MRR** (mean reciprocal rank)
3. Assert thresholds: precision@5 ≥ 0.6 for "easy" queries (exact subject_type match), ≥ 0.4 overall
4. Ensure ranking is deterministic: `ORDER BY score DESC, d.id ASC` provides stable tie-break

---

## 11. `whats-next` Enhancement

### 11.1 What Decision Traces Add

Today `whats-next` surfaces *what is open/pending*. Decision traces let it surface *how similar situations were resolved before*.

### 11.2 Augmented Output Shape

For each open task and pending hypothesis, append a `precedents` array:

```json
{
    "open_tasks": [
        {
            "id": "tsk-12",
            "title": "Implement FTS for studies",
            "status": "open",
            "precedents": [
                {
                    "decision_id": "dec-007",
                    "question": "How should FTS be implemented for findings?",
                    "chosen": "porter stemming via FTS5 triggers",
                    "because": "validated in spike 0.2; triggers auto-sync on INSERT",
                    "confidence": "high",
                    "session_id": "ses-prev",
                    "created_at": "2026-02-08T14:00:00Z"
                }
            ]
        }
    ],
    "pending_hypotheses": [
        {
            "id": "hyp-3",
            "content": "ast-grep patterns are fragile for Rust generics",
            "status": "unverified",
            "precedents": [
                {
                    "decision_id": "dec-012",
                    "question": "Are ast-grep patterns reliable for Rust extraction?",
                    "chosen": "use KindMatcher as primary, patterns only for specific queries",
                    "because": "spike 0.8 proved patterns miss return types and generics",
                    "confidence": "high",
                    "session_id": "ses-prev2",
                    "created_at": "2026-02-07T10:00:00Z"
                }
            ]
        }
    ],
    "recent_audit": [ ... ]
}
```

### 11.3 Query Approach

- Keep existing `whats_next()` base queries unchanged
- For each returned subject entity, build FTS query from title/content keywords
- Run precedent search SQL (Section 10.1) with `LIMIT 3`
- N+1 pattern, but N is small and bounded by `whats-next` output caps

---

## 12. Phase B: Graph Analytics Engine (`rustworkx-core`)

### 12.1 Graph Construction Pattern

1. Query bounded node/edge sets from `entity_links` + `decision_outcomes`
2. Deterministically sort nodes and edges by stable keys
3. Build in-memory `DiGraph<String, ()>`
4. Run algorithms
5. Return results with stable tie-break sorting

### 12.2 Algorithms to Validate

| Algorithm | Use case | Existing SQL alternative |
|---|---|---|
| `toposort` (DAG ordering) | Task dependency ordering for PRD | Recursive CTE — slower, less readable |
| Cycle detection | Validate task dependency DAGs | SQL — awkward for cycles |
| Betweenness centrality | "Most influential finding" in evidence network | No equivalent |
| Shortest path | Explainability: "evidence chain from decision to leaf finding" | Multi-join — brittle at depth > 3 |
| Weakly connected components | "Which decisions are related?" cluster detection | No equivalent |

### 12.3 Deterministic Tie-Break Policy

After algorithm scoring:
- Primary: score descending
- Secondary: entity kind order (insight > decision > finding > hypothesis > task > research)
- Tertiary: id ascending (lexicographic)

### 12.4 Budget Caps

- `max_nodes`: default 500, configurable
- `max_edges`: default 2000, configurable
- `max_depth`: default 10, configurable

When exceeded: stop expansion, return results with `truncated: true` + `truncation_reason`.

---

## 13. Test Matrix

### A. Decision Schema + Persistence (`zen-db`)

1. `spike_decision_create_roundtrip` — create via composite op, read back, verify all columns
2. `spike_decision_options_persisted` — options with correct `is_chosen`, `sort_order`
3. `spike_decision_option_evidence_persisted` — per-option evidence links stored
4. `spike_decision_outcomes_persisted` — outcome entities with typed relations
5. `spike_decision_entity_links_created` — entity_links mirror outcomes + evidence
6. `spike_decision_search_text_built_correctly` — denormalized FTS text matches expected shape
7. `spike_decision_fts_matches_question` — FTS finds by question keywords
8. `spike_decision_fts_matches_because` — FTS finds by justification keywords
9. `spike_decision_fts_matches_option_label` — FTS finds by option text
10. `spike_decision_fts_excludes_json_noise` — no JSON punctuation in FTS matches
11. `spike_decision_unique_chosen_constraint` — only one chosen option per decision enforced
12. `spike_decision_nullable_fields_correct` — policy/exception/approver NULL handling

### B. Replay + Versioning (`zen-db`)

13. `spike_decision_replay_roundtrip` — create → trail → rebuild → verify full parity across all 4 tables
14. `spike_decision_replay_strict_rejects_invalid` — malformed `data` payload fails strict replay
15. `spike_decision_replay_null_vs_absent` — JSON null = set NULL, absent = skip
16. `spike_decision_replay_old_trails_without_decisions` — old trails replay cleanly (decisions table stays empty)
17. `spike_decision_mutation_protocol_order` — BEGIN → SQL → audit → trail → COMMIT
18. `spike_decision_trail_failure_rolls_back` — partial trail append → no orphaned rows in any of the 4 tables

### C. Relation Enum (`zen-db`)

19. `spike_follows_precedent_link_created` — Decision → Decision with `FollowsPrecedent`
20. `spike_overrides_policy_link_created` — Decision → Insight with `OverridesPolicy`
21. `spike_supersedes_vs_overrides_distinction` — Supersedes marks old decision non-current; OverridesPolicy keeps policy active
22. `spike_derived_from_scoped_by_entity_type` — DerivedFrom queries filtered by (source_type, target_type) avoid collision with study-originated DerivedFrom links

### D. Precedent Search + Flywheel (`zen-db`)

23. `spike_precedent_search_finds_same_subject_type` — verdict decision about hypothesis found when querying for hypothesis
24. `spike_precedent_search_ranked_by_score` — shared evidence + confidence + recency all affect ranking
25. `spike_precedent_search_precision_at_5` — gold set evaluation, precision@5 ≥ 0.6 for exact subject_type match
26. `spike_precedent_search_deterministic_ranking` — same query → same order across runs
27. `spike_flywheel_new_trace_found_as_precedent` — create trace A, then search for it as precedent for trace B
28. `spike_flywheel_more_traces_improve_retrieval` — 5 traces → better precision than 1 trace for similar query
29. `spike_flywheel_cross_session_precedent` — trace from session 1 found by search in session 2

### E. Per-Option Queries (RQ5) (`zen-db`)

30. `spike_query_rejected_options_with_evidence` — "show decisions where rejected option had ≥ 2 evidence items"
31. `spike_query_same_alternative_evaluated` — "find precedents where option label 'debunk' was considered"
32. `spike_query_chosen_option_evidence_chain` — follow chosen option → evidence entities → their source

### F. `whats-next` Enhancement (`zen-db`)

33. `spike_whats_next_includes_precedent_for_open_task` — open task → finds prior decision about similar task
34. `spike_whats_next_includes_precedent_for_pending_hypothesis` — pending hypothesis → finds prior verdict decision
35. `spike_whats_next_surfaces_unresolved_exceptions` — exceptions without follow-up flagged

### G. Supersession (`zen-db`)

36. `spike_decision_superseded_by_new_decision` — new decision links to old via Supersedes
37. `spike_superseded_decision_excluded_from_precedent_search` — superseded decisions ranked lower or excluded

### H. Graph Algorithms — Phase B (`zen-search`)

38. `spike_task_dag_toposort_deterministic` — toposort on task depends-on graph
39. `spike_task_dag_cycle_detection` — cycle detected and reported
40. `spike_task_dag_ready_set` — nodes with all deps resolved
41. `spike_evidence_graph_centrality_ranking_stable` — betweenness centrality on evidence network
42. `spike_evidence_graph_shortest_explain_path` — shortest path from decision to leaf evidence
43. `spike_decision_graph_connected_components` — related decisions cluster together
44. `spike_graph_budget_max_nodes_enforced` — stops at cap, returns truncation metadata
45. `spike_graph_budget_max_edges_enforced`
46. `spike_graph_budget_max_depth_enforced`
47. `spike_graph_output_tie_break_stability` — repeated runs produce identical order/hash

### I. Visibility Safety (`zen-db`)

48. `spike_visibility_filter_before_graph_build` — filtered nodes/edges only
49. `spike_team_scope_does_not_leak_private_decisions`
50. `spike_public_scope_excludes_team_private_edges`

### J. Performance

51. `spike_perf_small_graph` — 500 nodes / 2k edges
52. `spike_perf_medium_graph` — 5k nodes / 20k edges
53. `spike_perf_large_graph` — 20k nodes / 100k edges
54. `spike_perf_deterministic_hash_across_runs`

Target: **54/54 passing**.

---

## 14. Crates and Files Touched

### Crates
- `zen-db` (decision schema, persistence, precedent search, replay, whats-next)
- `zen-search` (graph algorithms via `rustworkx-core`)
- `zen-core` (EntityType::Decision, Relation::FollowsPrecedent, Relation::OverridesPolicy — spike branch only)

### Spike Files
- `zen-db/src/spike_decision_traces.rs` — Phase A: schema, persistence, FTS, precedent search, flywheel, whats-next, supersession
- `zen-search/src/spike_graph_algorithms.rs` — Phase B: graph analytics
- `zen-db/tests/integration/spike_decision_e2e.rs` — visibility + performance

---

## 15. Fixture Design

### 15.1 Decision Trace Fixtures
- 20 decision traces across all 5 categories (verdict, architecture, planning, exception, completion)
- Distributed across 3 sessions
- 30% have precedent links (`FollowsPrecedent`) to prior traces
- 2 traces with `OverridesPolicy` links
- 2 traces superseded by later traces
- Mixed confidence levels (high/medium/low)

### 15.2 Options Fixtures
- Each decision has 2–4 options
- Exactly 1 chosen per decision (enforced by constraint)
- Options have 0–3 evidence entities each
- At least 3 decisions where the rejected option has more evidence than the chosen (interesting for RQ5)

### 15.3 Outcomes Fixtures
- 10 decisions with single outcome (e.g., validates one hypothesis)
- 5 decisions with multiple outcomes (e.g., confirms hypothesis + creates follow-up task)
- 5 decisions with no explicit outcome (architecture/planning decisions without immediate mutation)

### 15.4 Evidence Network Fixtures
- 50 findings linked to the 20 decisions via `DerivedFrom`
- 10 hypotheses as outcome targets
- 5 insights as policy references

### 15.5 Task DAG Fixtures
- Acyclic DAG fixture (expected toposort order known)
- Single-cycle fixture (for detection test)
- Multi-component fixture (for connected components)

### 15.6 Precedent Search Gold Set
- 15 queries with expected relevant decision IDs (3–5 per query)
- Mix of exact subject_type match, keyword-only, and evidence-overlap scenarios
- Used by tests 25, 26

### 15.7 Scale Fixtures (Performance)
- Synthetic node/edge generation at 500 / 5k / 20k node scales

---

## 16. Concrete Code Examples

### 16.1 Decision Creation (Rust — spike pattern)

```rust
let decision = SpikeDecision {
    id: db.generate_id("dec").await?,
    session_id: session_id.to_string(),
    category: "verdict".to_string(),
    subject_type: "hypothesis".to_string(),
    subject_id: hyp_id.to_string(),
    question: "Does tokio::spawn require Send + 'static?".to_string(),
    because: "3 independent code tests prove Send + 'static is required".to_string(),
    outcome_summary: Some("hypothesis confirmed".to_string()),
    policy_type: None,
    policy_id: None,
    exception_kind: None,
    exception_reason: None,
    approver: Some("llm".to_string()),
    confidence: "high".to_string(),
    metadata_json: None,
    created_at: Utc::now(),
    updated_at: Utc::now(),
};

let options = vec![
    SpikeDecisionOption {
        id: db.generate_id("opt").await?,
        decision_id: decision.id.clone(),
        label: "confirm".to_string(),
        summary: Some("E0277 error proves Send bound".to_string()),
        is_chosen: true,
        sort_order: 0,
        evidence: vec![
            (EntityType::Finding, fnd_a_id.clone()),
            (EntityType::Finding, fnd_b_id.clone()),
        ],
    },
    SpikeDecisionOption {
        id: db.generate_id("opt").await?,
        decision_id: decision.id.clone(),
        label: "inconclusive".to_string(),
        summary: Some("Could be test setup artifact".to_string()),
        is_chosen: false,
        sort_order: 1,
        evidence: vec![
            (EntityType::Finding, fnd_c_id.clone()),
        ],
    },
];

let outcomes = vec![
    SpikeDecisionOutcome {
        entity_type: EntityType::Hypothesis,
        entity_id: hyp_id.clone(),
        relation: Relation::Validates,
    },
];

// Single transaction: all tables + entity_links + audit + trail
spike_create_decision(&service, &decision, &options, &outcomes).await?;
```

### 16.2 Fixed INSERT with Nullable Fields

```rust
conn.execute(
    "INSERT INTO decisions (
        id, session_id, category, subject_type, subject_id,
        question, because, outcome_summary,
        policy_type, policy_id, exception_kind, exception_reason,
        approver, confidence, search_text, metadata_json,
        created_at, updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
    libsql::params![
        decision.id.as_str(),
        decision.session_id.as_str(),
        decision.category.as_str(),
        decision.subject_type.as_str(),
        decision.subject_id.as_str(),
        decision.question.as_str(),
        decision.because.as_str(),
        decision.outcome_summary.as_deref(),
        decision.policy_type.as_deref(),
        decision.policy_id.as_deref(),
        decision.exception_kind.as_deref(),
        decision.exception_reason.as_deref(),
        decision.approver.as_deref(),
        decision.confidence.as_str(),
        build_search_text(&decision, &options),
        decision.metadata_json.as_deref(),
        decision.created_at.to_rfc3339(),
        decision.updated_at.to_rfc3339(),
    ],
).await?;
```

### 16.3 Graph Build + Centrality (Phase B)

```rust
use rustworkx_core::centrality::betweenness_centrality;
use rustworkx_core::petgraph::graph::DiGraph;

let mut g: DiGraph<String, ()> = DiGraph::new();
// Insert sorted nodes/edges from entity_links + decision_outcomes queries...
let scores = betweenness_centrality(&g, false, false, 200);
```

### 16.4 DAG Ordering (Phase B)

```rust
use rustworkx_core::petgraph::algo::toposort;

let order = toposort(&g, None)
    .map_err(|e| anyhow::anyhow!("cycle at node {:?}", e.node_id()))?;
```

---

## 17. Execution Commands

```bash
# Phase A: Decision trace model + precedent search
cargo test -p zen-db spike_decision_traces -- --nocapture

# Phase B: Graph algorithms
cargo test -p zen-search spike_graph_algorithms -- --nocapture

# Integration tests (visibility + performance)
cargo test -p zen-db spike_decision_e2e -- --nocapture

# Final gate
cargo test -p zen-db -p zen-search
```

---

## 18. Success Criteria (Go / No-Go)

### Go if all true:

1. 54/54 spike tests pass
2. No regression of Phase 2 invariants (Section 4)
3. Precedent search precision@5 ≥ 0.6 for exact subject_type match queries
4. Per-option queries (tests 30-32) return correct, non-trivial results
5. Graph determinism confirmed by repeated-run hash equality
6. Budget caps enforce and report truncation
7. Composite trail op replays correctly across all 4 tables + entity_links
8. `whats-next` enhancement produces useful precedent snippets in tests 33-35
9. Performance acceptable at medium graph scale for interactive CLI

### No-Go if any true:

- Replay parity mismatch after rebuild (any of the 4 tables)
- Mutation protocol ordering breaks
- Precedent search returns only noise (precision@5 < 0.3)
- Non-deterministic graph output on repeated runs
- Visibility leakage through decision trace links or outcomes
- `rustworkx-core` performance unacceptable at medium scale
- Composite trail op produces partial state on crash

---

## 19. Risk Log

1. **Noise from over-instrumentation**
   - Mitigation: only 5 decision categories; everything else stays in audit trail
2. **Schema complexity (4 tables for one feature)**
   - Mitigation: `decision_options`, `decision_option_evidence`, `decision_outcomes` are simple join tables with no business logic; complexity is in queries, not schema
3. **Composite trail op increases replayer complexity**
   - Mitigation: replayer handles it as a single transaction; validated by tests 13-18
4. **Relation enum extension breaks existing code**
   - Mitigation: spike branch only; additions are pure enum variants, no existing match arms change
5. **Graph explosion**
   - Mitigation: hard budget caps + truncation metadata
6. **Nondeterministic outputs**
   - Mitigation: stable insertion + explicit tie-break sorting + `ORDER BY score DESC, d.id ASC`
7. **Scope creep into full policy/exception/approval engine**
   - Mitigation: only model what the spike tests require; no workflow orchestration

---

## 20. Deliverables

1. Spike code modules listed in Section 14
2. Passing test suite (54 tests)
3. Precedent search evaluation report (precision@5 + MRR per query category)
4. Benchmark notes (timing + determinism hashes)
5. Final recommendation: `Adopt directly` / `Adopt behind feature flag` / `Do not adopt`

Recommendation must cite test evidence by name.

---

## 21. Migration and Replay Compatibility

### 21.1 Migration

New migration `002_decisions.sql` creates:
- `decisions` + indexes
- `decision_options` + indexes + constraint
- `decision_option_evidence` + index
- `decision_outcomes` + index
- `decisions_fts` + triggers

### 21.2 Old Trail Compatibility

Old trails (without `decision_create` ops) replay cleanly — the new tables remain empty. This is not "unknown op" (which would hard-fail); it is simply the absence of decision ops.

### 21.3 Version Dispatch

Replay dispatch handles:
- `op.entity == "decision" && op.op == "decision_create"` → composite handler
- `op.entity == "decision" && op.op == "decision_update"` → patch handler
- All other ops → existing handlers unchanged

Unknown ops still hard-fail (strictness invariant preserved).

---

## 22. Post-Spike Integration (Informational Only)

If Go, integration tasks would land in:

- **Phase 2**: `002_decisions.sql` migration, DecisionRepo, replay handlers
- **Phase 4**: Graph assembly + precedent search engine in zen-search
- **Phase 5**: `znt decision create/get/list/graph/precedents` commands, `znt whats-next --precedents`, `--because` / `--evidence` flags on `znt hypothesis update`, `znt task complete`
- **Phase 6**: PRD DAG validation (toposort, cycle detection) via graph engine
- **Phase 9**: Visibility-safe team decision traces (scoped by `org_id`)

No existing plan files are modified by this spike.

---

## 23. Evidence Discipline

- Every behavior claim must cite a passing test name
- Any API assumption not validated by a test must be marked `UNVERIFIED`
- `spike_` tests are source of truth; do not infer behavior from memory
- Precedent search quality measured by precision@5 over gold set, not subjective judgment

---

## 24. Conceptual Mapping: Context Graphs → Zenith

| Context Graph concept | Zenith implementation |
|---|---|
| Decision trace | `decisions` table (first-class entity) |
| Inputs gathered | `decision_option_evidence` + `entity_links` (`DerivedFrom`) |
| Options considered | `decision_options` table with `is_chosen` flag |
| Policy evaluated | `decisions.policy_type/policy_id` + `entity_links` |
| Exception invoked | `decisions.exception_kind/exception_reason` + `OverridesPolicy` relation |
| Approver | `decisions.approver` |
| Outcome | `decision_outcomes` table + `entity_links` (`Validates`, `Implements`, etc.) |
| Precedent links | `entity_links` with `FollowsPrecedent` relation |
| Compounding flywheel | Precedent search in `whats-next` (tests 27-29, 33-35) |
| Decision-time snapshot | Per-option evidence captures what was known; options + chosen captures what was considered |
| Explainability chain | Shortest path algorithm (test 42) |
| Case law system | FTS + graph centrality over accumulated decision traces; superseded decisions marked, not deleted |
| "Why not option B?" | `decision_option_evidence` query on rejected options (tests 30-32) |

### What This Spike Intentionally Does NOT Model

- Multi-step approval chains (no approver workflow — the LLM decides, the human says "Go")
- Policy engines / formal rule evaluation (policies are informal Insights/Findings)
- Cross-graph federation across multiple Zenith instances
- Automated decision trace extraction from LLM chain-of-thought
- Privacy/compliance (redaction, right-to-forget) — deferred to Phase 9 visibility work
- `DecisionOption` as a first-class EntityType (options are sub-rows, not globally addressable)

These are not needed to prove the core concept. They can be added if the spike succeeds.
