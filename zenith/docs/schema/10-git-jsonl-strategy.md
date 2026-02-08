# Zenith: Git & JSONL Strategy

**Version**: 2026-02-08
**Status**: **DONE** — Approach B (JSONL as source of truth) selected. `serde-jsonlines` confirmed.
**Purpose**: Design the JSONL audit trail, git integration hooks, and investigate whether rebuild-from-JSONL is worth the complexity
**Spike ID**: 0.12
**Crate**: zen-db

## Spike Results

**Decision: Approach B (JSONL as source of truth) wins. `serde-jsonlines` selected as the JSONL crate.**

All 15 tests pass. Key findings:

| Dimension | A (export only) | B (source of truth) |
|-----------|:---:|:---:|
| DB rebuildable from JSONL? | No | **Yes** |
| Survives DB corruption? | No | **Yes** |
| git clone gives full state? | No | **Yes** |
| Turso Cloud required? | For durability | **Optional** |
| JSONL entry size | ~155 B | ~220 B |
| Replay logic | None | **~60 LOC** |
| FTS5 after rebuild? | N/A | **Works** |
| Entity links after rebuild? | N/A | **Works** |

**Why Approach B wins:**
- DB becomes disposable — delete it, replay from JSONL, get identical state back
- `git clone` + `zen rebuild` gives you the full project knowledge base on a new machine
- Turso Cloud sync becomes optional for durability (git is the backup)
- The replay logic is ~60 lines of Rust, same maintenance cost as SQL migrations
- FTS5 indexes and entity_links both survive rebuild (tested)
- Per-session JSONL files are naturally concurrent-safe (4 agents, 100 ops, zero corruption)

**Why `serde-jsonlines`:**
- `append_json_lines()` — 1 line vs 4 for raw `serde_json`
- `json_lines()` — batch read with iterator, 1 line vs 4
- Built-in file creation, error handling
- ~9,300 downloads/month, actively maintained (v0.7.0)

---

## Table of Contents

1. [Overview](#1-overview)
2. [Design Decisions (Settled)](#2-design-decisions-settled)
3. [Open Question: Rebuild From JSONL](#3-open-question-rebuild-from-jsonl)
4. [JSONL Crate Selection](#4-jsonl-crate-selection)
5. [JSONL Format Specification](#5-jsonl-format-specification)
6. [File Structure](#6-file-structure)
7. [Git Integration](#7-git-integration)
8. [Multi-Agent Concurrency](#8-multi-agent-concurrency)
9. [Spike Plan](#9-spike-plan)
10. [Post-Spike Actions](#10-post-spike-actions)

---

## 1. Overview

Zenith needs a git-friendly representation of its knowledge state. The SQLite database (`.zenith/zenith.db`) is binary and produces useless diffs — it should not be committed to git. Instead, a JSONL (JSON Lines) append-only format provides:

- **Human-readable** audit trail in git history
- **Merge-safe** format (append-only minimizes conflicts)
- **Portable** state representation across machines
- **Potentially**: a source of truth from which the DB can be rebuilt

### Prior Art: Beads 3-Layer Architecture

Beads uses a proven 3-layer pattern (from `reference/beads/architecture/`):

```
Layer 1: Git Repository    <- Historical source of truth
Layer 2: JSONL Files       <- Operational source of truth (append-only)
Layer 3: SQLite Database   <- Fast queries / derived state (rebuildable)
```

Key beads insight: **SQLite is disposable**. Delete it, run `bd sync --import-only`, and it rebuilds from JSONL. This makes the system extremely resilient — corruption, migration issues, and machine transfers are all solved by "rebuild from JSONL."

The open question is whether this pattern scales to Zenith's richer entity model (15+ tables, FTS5, entity_links) or whether the rebuild complexity outweighs the benefit.

### Prior Art: Workflow Tool Plan

The `workflow-tool-plan/07-git-integration-strategy.md` specifies `gix` (pure Rust git), agent branch isolation, and per-agent JSONL files. This is over-engineered for Zenith's single-tool CLI model but informs the multi-agent concurrency design.

---

## 2. Design Decisions (Settled)

These decisions were made during brainstorming and don't need spike validation:

| Decision | Choice | Reasoning |
|----------|--------|-----------|
| Git operations | **Not our responsibility** | User/LLM handles `git add`/`commit`. We produce files and provide hooks. |
| What to `.gitignore` | `zenith.db`, `zenith.db-wal`, `zenith.db-shm`, `*.db-journal` | Binary files don't belong in git |
| What to track in git | `.zenith/trail/*.jsonl`, `.zenith/hooks/`, `.zenith/config.toml` | Human-readable, merge-safe |
| Multi-agent model | Per-session isolation | Each LLM instance gets its own session, writes to its own JSONL file |
| JSONL trigger | Real-time append (every mutation) | Durable — survives crashes before wrap-up |
| Git library | None (we don't do git ops) | We provide hooks as shell scripts, user installs them |

---

## 3. Open Question: Rebuild From JSONL

This is the central question the spike must answer.

### Approach A: JSONL as Export Only

JSONL is an append-only audit log. The SQLite database is the source of truth. JSONL is useful for:
- Git history and diffs
- Human review of what happened in a session
- Cross-machine sharing of session logs

But you **cannot** rebuild the DB from JSONL. If the DB is lost, the knowledge is lost (unless you have a cloud sync via Turso).

**Pros**: Simpler format (just audit entries), no replay logic, smaller JSONL files
**Cons**: DB corruption = data loss, can't bootstrap from git clone, Turso Cloud becomes mandatory for durability

### Approach B: JSONL as Source of Truth (Beads Pattern)

Every mutation is recorded as an **operation** in JSONL. The SQLite DB is a materialized view that can be rebuilt by replaying the JSONL. Recovery sequence: `rm zenith.db && zen rebuild`.

**Pros**: DB is disposable, git clone gives you full state, no cloud dependency for durability, machine transfer is trivial
**Cons**: JSONL format must capture full mutation state (not just "what changed" but "what the new values are"), replay logic for 15+ tables + FTS5 + entity_links, larger JSONL files, more complex write path

### What the Spike Validates

1. **Can we define a JSONL operation format that covers all 15+ entity types?**
2. **Can we replay operations to rebuild a correct DB state?** (Including FTS5 indexes and entity_links)
3. **What's the performance cost?** Rebuild 1000 operations, 5000 operations
4. **What's the format size difference?** Export-only vs full-operation per mutation
5. **Is the replay logic manageable?** Or does it become a maintenance burden with every schema change?

---

## 4. JSONL Crate Selection

Research identified these candidates:

| Crate | Batch Read | Append | Async | Downloads/mo | Notes |
|-------|-----------|--------|-------|-------------|-------|
| `serde-jsonlines` | Yes (iterator → collect) | Yes (`append_json_lines()`) | Optional | ~9,300 | Best ergonomics, most popular |
| `json-lines` | Via tokio codec | Via codec | Yes | ~350 | `no_std`, codec-oriented |
| `jsonl` | Line by line | No dedicated | Optional | Low | Minimal API |
| `serde_json` (raw) | Manual loop | Manual `writeln!` | No | Already dep | Zero extra deps |

**Spike will test**: `serde-jsonlines` vs raw `serde_json` to compare ergonomics. If `serde-jsonlines` is significantly better, add it as a dependency. If the difference is marginal, stick with `serde_json` to avoid a new dep.

Key `serde-jsonlines` features to evaluate:
- `append_json_lines(path, &items)` — appends to file, creating if needed
- `json_lines(path)?.collect::<io::Result<Vec<T>>>()` — batch read
- `BufReadExt::json_lines()` — streaming iterator from any `BufRead`
- `WriteExt::write_json_lines()` — streaming write to any `Write`

---

## 5. JSONL Format Specification

### Operation-Based Format (for Approach B)

Each line is a self-contained operation that can be replayed:

```jsonl
{"ts":"2026-02-08T10:00:00Z","ses":"ses-001","op":"create","entity":"session","id":"ses-001","data":{"status":"active"}}
{"ts":"2026-02-08T10:01:00Z","ses":"ses-001","op":"create","entity":"research","id":"res-001","data":{"title":"Study: How tokio::spawn works","status":"in_progress"}}
{"ts":"2026-02-08T10:02:00Z","ses":"ses-001","op":"create","entity":"hypothesis","id":"hyp-001","data":{"research_id":"res-001","content":"spawn requires Send","status":"unverified"}}
{"ts":"2026-02-08T10:05:00Z","ses":"ses-001","op":"update","entity":"hypothesis","id":"hyp-001","data":{"status":"confirmed","reason":"E0277 proves it"}}
{"ts":"2026-02-08T10:06:00Z","ses":"ses-001","op":"create","entity":"entity_link","id":"lnk-001","data":{"source_type":"finding","source_id":"fnd-001","target_type":"hypothesis","target_id":"hyp-001","relation":"validates"}}
```

Fields:
- `ts`: ISO 8601 timestamp
- `ses`: Session ID (which session produced this operation)
- `op`: Operation type (`create`, `update`, `delete`)
- `entity`: Entity type (`session`, `research`, `finding`, `hypothesis`, `insight`, `issue`, `task`, `impl_log`, `compat`, `study`, `entity_link`, `finding_tag`, `audit`)
- `id`: Entity ID
- `data`: Full entity data for creates, changed fields for updates

### Audit-Only Format (for Approach A)

Simpler — just mirrors the audit_trail table:

```jsonl
{"ts":"2026-02-08T10:01:00Z","ses":"ses-001","entity":"research","id":"res-001","action":"create","detail":"Created research: Study: How tokio::spawn works"}
{"ts":"2026-02-08T10:05:00Z","ses":"ses-001","entity":"hypothesis","id":"hyp-001","action":"update","detail":"Status changed: unverified -> confirmed"}
```

---

## 6. File Structure

```
.zenith/
├── zenith.db              # SQLite (gitignored)
├── zenith.db-wal          # WAL file (gitignored)
├── config.toml            # User config (git tracked)
├── .gitignore             # Ignores DB files
├── hooks/                 # Git hooks (git tracked)
│   ├── pre-commit         # Validates JSONL format
│   └── post-checkout      # Rebuilds DB if needed (Approach B only)
└── trail/                 # JSONL files (git tracked)
    ├── ses-test0001.jsonl # Per-session trail
    ├── ses-abc12345.jsonl # Another session
    └── ...
```

**Per-session JSONL files** because:
- Natural scoping (one file per unit of work)
- Merge-safe (different sessions = different files = no conflicts)
- Concurrent-safe (multiple agents = multiple sessions = multiple files)
- Easy to review per-session
- Easy to archive/compact old sessions

---

## 7. Git Integration

### What We Provide

1. **`.gitignore` template** generated by `zen init`:
   ```
   # Zenith database (rebuildable from JSONL trail)
   zenith.db
   zenith.db-wal
   zenith.db-shm
   *.db-journal
   ```

2. **Pre-commit hook** (`hooks/pre-commit`):
   - Validates all `.jsonl` files in `.zenith/trail/` are valid JSONL
   - Rejects commits with malformed JSON lines

3. **Post-checkout hook** (Approach B only, `hooks/post-checkout`):
   - After branch switch, checks if JSONL files changed
   - If so, triggers `zen rebuild` to reconstruct the DB

### What We Do NOT Provide

- `git add` / `git commit` / `git push` — user's responsibility
- Branch management — user's responsibility
- Merge conflict resolution — the append-only format prevents most conflicts; for the rare case, "keep both lines" is always correct

---

## 8. Multi-Agent Concurrency

Users may run multiple LLM agents (e.g., multiple Claude Code instances) simultaneously. Each agent:

1. Starts its own Zenith session (`zen session start` → unique `ses-xxx` ID)
2. Writes to its own JSONL file (`.zenith/trail/ses-xxx.jsonl`)
3. Writes to the shared SQLite DB (libsql handles concurrent writes via WAL mode)

**No cross-session file contention**: each agent appends to its own JSONL file.

**SQLite concurrency**: WAL mode supports concurrent reads + single writer. Multiple agents may experience brief write contention but libsql handles this with retry logic.

**Git safety**: When agents commit, per-session files mean no merge conflicts between concurrent agents on the same branch.

---

## 9. Spike Plan

### Spike 0.12: JSONL Trail Validation

**File**: `zenith/crates/zen-db/src/spike_jsonl.rs`

**Scenario**: Same as spike 0.11 — "Learn how tokio::spawn works" — but this time we write operations to JSONL and (for Approach B) rebuild the DB from JSONL.

### Part A: Crate Comparison (3 tests)

Test `serde-jsonlines` vs raw `serde_json` for JSONL operations:

| # | Test | What It Validates |
|---|------|-------------------|
| 1 | `spike_jsonl_serde_jsonlines_roundtrip` | Write 10 operations with `append_json_lines()`, read back with `json_lines()`, verify all 10 round-trip correctly |
| 2 | `spike_jsonl_raw_serde_json_roundtrip` | Same 10 operations with raw `serde_json::to_string()` + `writeln!()`, read with `BufReader::lines()` + `from_str()`, verify round-trip |
| 3 | `spike_jsonl_compare_ergonomics` | Side-by-side comparison: lines of code for write, read, append. Print comparison table. |

### Part B: Approach A — Export Only (3 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 4 | `spike_jsonl_audit_export` | Create entities in SQLite, export audit trail to JSONL. Verify format and content. |
| 5 | `spike_jsonl_audit_read_back` | Read exported JSONL, verify all audit entries are present and parseable. |
| 6 | `spike_jsonl_audit_size` | Measure JSONL size for 100 audit entries. Extrapolate to 10K. |

### Part C: Approach B — Source of Truth (6 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 7 | `spike_jsonl_operation_format` | Define the operation enum (Create/Update/Delete) with serde. Serialize/deserialize all 15 entity types. |
| 8 | `spike_jsonl_write_operations` | Run the full study scenario (create session, research, hypotheses, findings, insights, links). Write each mutation as an operation to JSONL. |
| 9 | `spike_jsonl_replay_rebuild` | Read the JSONL from test 8. Replay operations into a fresh in-memory DB. Verify the rebuilt DB matches the original. |
| 10 | `spike_jsonl_rebuild_fts` | After replay, verify FTS5 indexes work (search for "tokio spawn" returns results). |
| 11 | `spike_jsonl_rebuild_entity_links` | After replay, verify entity_links are correct (finding validates hypothesis). |
| 12 | `spike_jsonl_operation_size` | Measure JSONL size for the full scenario. Compare to Approach A's audit-only size. |

### Part D: Comparison (1 test)

| # | Test | What It Validates |
|---|------|-------------------|
| 13 | `spike_jsonl_compare_approaches` | Print comparison: format size, lines of replay code, rebuild correctness, maintenance burden estimate. |

### Part E: Per-Session Files + Concurrent Append (2 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 14 | `spike_jsonl_per_session_files` | Write operations to two separate session files. Read both back. Verify isolation. |
| 15 | `spike_jsonl_concurrent_append` | Spawn 4 tokio tasks appending to separate session files simultaneously. Verify no corruption. |

**Total: 15 tests**

### Success Criteria

- Both approaches compile and all tests pass
- Clear data on format size (A vs B)
- Rebuild from JSONL either works correctly (including FTS5) or we document exactly why it doesn't
- Crate recommendation: `serde-jsonlines` or raw `serde_json`
- Decision: Approach A or B, with evidence

---

## 10. Post-Spike Actions

### If Approach B Wins (JSONL as Source of Truth)

1. Update `01-turso-data-model.md`: document JSONL operation format alongside SQL schema
2. Update `05-crate-designs.md`: add JSONL write/replay to zen-db
3. Update `07-implementation-plan.md`: add JSONL layer to Phase 2 or Phase 8
4. Add `zen rebuild` command to `04-cli-api-design.md`
5. Update `INDEX.md`
6. Turso Cloud sync becomes optional (JSONL in git is the durable backup)

### If Approach A Wins (JSONL as Export Only)

1. Update `01-turso-data-model.md`: document audit export format
2. JSONL export stays in Phase 8 (wrap-up only enhancement)
3. Turso Cloud remains important for cross-machine durability
4. Update `INDEX.md`

### Regardless of Outcome

1. Add `serde-jsonlines` to workspace deps (if spike confirms it's worth it)
2. Implement `.gitignore` template in `zen init`
3. Implement pre-commit hook for JSONL validation
4. Document per-session file structure

---

## Cross-References

- Turso data model: [01-turso-data-model.md](./01-turso-data-model.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md) (Phase 8, tasks 8.4-8.5)
- Studies spike: [08-studies-spike-plan.md](./08-studies-spike-plan.md) (same scenario reused)
- Reference: Beads JSONL layer — `reference/beads/architecture/jsonl-layer.md`
- Reference: Beads Git layer — `reference/beads/architecture/git-layer.md`
- Reference: Beads architecture overview — `reference/beads/architecture/overview.md`
- Prior planning: `workflow-tool-plan/07-git-integration-strategy.md`
