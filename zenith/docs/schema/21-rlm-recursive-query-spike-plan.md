# Zenith: Recursive Context Query (RLM) — Spike Plan

**Version**: 2026-02-09
**Status**: DONE -- 17/17 tests pass
**Purpose**: Validate an RLM-style “prompt-as-environment” workflow over a large codebase (Apache Arrow Rust monorepo) using symbolic access (AST + doc comment spans + source cache), recursive sub-queries, and output assembly without lossy compaction.
**Spike ID**: 0.21
**Crate**: zen-search (primary), zen-parser (AST helpers), zen-lake (source cache input)
**Blocks**: Phase 4 (Search & Registry), Phase 5 (CLI search behavior)

---

## Spike Results

**Decision: RLM-style recursive query flow is viable for Zenith search.**

Implemented and validated in `zenith/crates/zen-search/src/spike_recursive_query.rs` on the full Arrow Rust monorepo.

Key outcomes:
- **Scale validated on real monorepo**: 606 Rust files, 407,210 lines, 14,929,705 bytes.
- **Extended impl query works**: captures trait impl + generic impl patterns and outperforms baseline (`delta = +580` matches across repo).
- **Budgeted recursion works**: metadata-only planning and chunk/byte caps pass deterministically.
- **Reference categorization works**: `same_module`, `other_module_same_crate`, `other_crate_workspace`, `external`.
- **External references validated**: Arrow usage discovered in cached DataFusion crates (`~/.cargo/registry/src/**/datafusion-*`).
- **Signature preservation + lookup works**: each hit has a stable `ref_id` and signature lookup path.
- **Reference graph persistence works**: in-memory DuckDB tables (`symbol_refs`, `ref_edges`) store refs/edges and support category stats + signature lookup.
- **Machine-friendly reporting added**: compact and pretty JSON summaries (`[summary_json]`, `[summary_json_pretty]`).

---

## Table of Contents

1. [Spike Results](#spike-results)
2. [Motivation](#1-motivation)
3. [Background & RLM Principles](#2-background--rlm-principles)
4. [Scope & Non-Goals](#3-scope--non-goals)
5. [Target Dataset (Arrow Monorepo)](#4-target-dataset-arrow-monorepo)
6. [Proposed Design](#5-proposed-design)
7. [Search & Extraction Patterns](#6-search--extraction-patterns)
8. [Spike Scenario](#7-spike-scenario)
9. [Spike Tests](#8-spike-tests)
10. [Evaluation Criteria](#9-evaluation-criteria)
11. [Success Criteria](#10-success-criteria)
12. [Post-Spike Actions](#11-post-spike-actions)
13. [Cross-References](#12-cross-references)

---

## 1. Motivation

Zenith must reason over large codebases and large documentation sets without losing fidelity. RLMs show that long-context tasks degrade when we treat the full prompt as a single context window, and that **symbolic access + recursive sub-calls** avoid this failure mode. This spike validates that approach at real scale using the Apache Arrow Rust monorepo.

---

## 2. Background & RLM Principles

From `reference/llm-context-management/recursive_language_models.md`:

- **Prompt-as-environment**: full input is accessed symbolically, not ingested.
- **Symbolic handles**: file paths, AST node spans, doc spans.
- **Recursive sub-calls**: programmatic invocation on slices.
- **Output assembly**: stitched from sub-results, not monolithic.
- **Metadata-only root loop**: constant-size history, no compaction.

---

## 3. Scope & Non-Goals

**In-scope**
- A deterministic recursive query loop over cached Arrow source.
- AST + doc comment span extraction used as symbolic handles.
- Filter-first pipeline: regex/AST selection before sub-calls.
- Budgeted recursion: depth, chunk count, byte caps.
- Output assembly from sub-results.

**Out-of-scope**
- Real LLM calls (use deterministic mock sub-call).
- Full indexing pipeline or embeddings.
- CLI integration beyond test harness.
- Performance optimization beyond bounded budgets.

---

## 4. Target Dataset (Arrow Monorepo)

**Dataset Path**
`/Users/wrath/reference/rust/arrow-rs`

**Scope**
All workspace crates listed in `/Users/wrath/reference/rust/arrow-rs/Cargo.toml` (entire monorepo). Rust source files only: `**/*.rs`.

**Optional exclusions**
- Generated code: `arrow-ipc/src/gen/**`
- Tests/benches/examples: `**/tests/**`, `**/benches/**`, `**/examples/**`

Default: include everything for scale realism.

---

## 5. Proposed Design (Spike-Only)

### Core components

- **ContextStore**
  In-memory map of `file_path → source_text + AST spans + doc spans`.
- **ChunkSelector**
  Produces slices using:
  - AST query results
  - doc-comment keyword scan
  - regex over source cache
- **RecursiveQueryEngine**
  - Root loop sees **metadata only** (counts, file list, total spans)
  - Runs filter stage to pick slices
  - Invokes `sub_call(slice)` for each slice (deterministic mock)
  - Aggregates sub-results into final output

### Budgets

- `max_depth`
- `max_chunks`
- `max_bytes_per_chunk`
- `max_total_bytes`

---

## 6. Search & Extraction Patterns

### ast-grep (primary, from `zen-parser` spike)

- **KindMatcher-first** extraction for Rust nodes.
- **Doc comment extraction** via sibling `prev()` walk (klaw pattern).

### tree-sitter (secondary/verification, from klaw-effect-tracker)

Baseline Rust queries:

```scm
(function_item name: (identifier) @name) @function
(struct_item name: (type_identifier) @name) @struct
(enum_item name: (type_identifier) @name) @enum
(trait_item name: (type_identifier) @name) @trait
(impl_item type: (type_identifier) @name) @impl
```

**Extended impl queries (required for this spike)**

Trait impls:

```scm
(impl_item
  trait: (type_identifier) @trait
  type: (type_identifier) @name) @impl
```

```scm
(impl_item
  trait: (scoped_type_identifier) @trait
  type: (type_identifier) @name) @impl
```

Generic impls:

```scm
(impl_item
  type: (generic_type
          type: (type_identifier) @name)) @impl
```

```scm
(impl_item
  type: (scoped_type_identifier) @name) @impl
```

Trait + generic type:

```scm
(impl_item
  trait: (type_identifier) @trait
  type: (generic_type
          type: (type_identifier) @name)) @impl
```

```scm
(impl_item
  trait: (scoped_type_identifier) @trait
  type: (generic_type
          type: (type_identifier) @name)) @impl
```

---

## 7. Spike Scenario (Arrow Monorepo)

### Linear task (O(n))
“Find all public APIs whose doc comments mention **safety**, **panic**, or **invariant**. Return file paths + symbol names + doc excerpt.”

### Pairwise task (O(n²))
“Find `(type, function)` reference pairs with shared semantic themes in docs, then categorize locality (`same_module`, `other_module_same_crate`, `other_crate_workspace`, `external`).”

### Budgeted recursion
Run both tasks with:
- `max_chunks = 200`
- `max_bytes_per_chunk = 6_000`
- `max_total_bytes = 750_000`
- `max_depth = 2`

---

## 8. Spike Tests

**File**: `zenith/crates/zen-search/src/spike_recursive_query.rs`

### Part A — Dataset & indexing

1. `spike_arrow_repo_scan`
   Build `ContextStore` from Arrow monorepo; log counts (files, symbols, doc spans).

2. `spike_doc_span_extraction`
   Verify doc comment extraction on real files (at least 3 crates).

3. `spike_ast_tree_preview`
   Print a truncated tree-sitter S-expression preview for a real Arrow file to verify AST accessibility.

### Part B — Pattern correctness

4. `spike_impl_query_trait_uuid`
   Ensure extended impl query captures:
   `impl ExtensionType for Uuid` in `arrow-schema/src/extension/canonical/uuid.rs:40`.

5. `spike_impl_query_generic_fields`
   Ensure extended impl query captures:
   `impl<const N: usize> From<[FieldRef; N]> for Fields` in `arrow-schema/src/fields.rs:97`.

6. `spike_impl_query_delta`
   Compare baseline impl query vs extended query counts across Arrow; assert extended ≥ baseline.

### Part C — Recursive query behavior

7. `spike_recursive_metadata_only_root_and_budget`
   Root loop sees metadata only (no raw source in root context).

8. `spike_recursive_filter_ast_and_docs`
   AST + doc comment filters select expected slices.

9. `spike_recursive_sub_call_dispatch`
   Selected slices produce deterministic sub-call outputs.

10. `spike_recursive_output_assembly`
   Output is stitched in stable order with file path + symbol name.

### Part D — Budget enforcement

11. `spike_recursive_budget_max_chunks`
    Selection truncates deterministically at `max_chunks`.

12. `spike_recursive_budget_max_bytes`
    Slice truncation honors `max_bytes_per_chunk`.

13. `spike_recursive_budget_total`
    Hard cap stops recursion without error.

### Part E — Task correctness

14. `spike_recursive_linear_task`
    “safety/panic/invariant” task returns complete, non-empty results.

15. `spike_recursive_pairwise_task`
    Pairwise task returns stable categorized reference pairs and external DataFusion reference samples.

### Part F — Determinism

16. `spike_recursive_stability`
    Two runs yield identical output.

### Part G — Persistence & lookup

17. `spike_reference_graph_persistence`
    Persist symbol refs and edges into DuckDB (`symbol_refs`, `ref_edges`), validate signature lookup by `ref_id`, and print category counts.

**Total: 17 tests**

---

## 9. Evaluation Criteria

| Criterion | Weight | Measurement |
|---|---|---|
| Correctness on real monorepo | High | Linear + categorized pairwise tasks return expected results |
| Pattern coverage | High | Extended impl queries capture known trait + generic impls |
| Budget adherence | High | No run exceeds byte or chunk caps |
| Determinism | Medium | Stable output across runs |
| Symbolic access | High | Root context metadata-only; no compaction |
| Signature fidelity | High | Hit-level signatures and `ref_id -> signature` lookup succeed |
| Reference graph persistence | Medium | `symbol_refs`/`ref_edges` insert + category stats succeed |

---

## 10. Success Criteria

- All 17 tests pass.
- Extended impl queries capture `Uuid` and `Fields` examples.
- Linear and pairwise tasks return results without full-context ingestion.
- Budgets enforce predictable bounds.
- Reference categories are produced (`same_module`, `other_module_same_crate`, `other_crate_workspace`, `external`).
- Summary JSON output includes pair samples, external samples, and signatures.

---

## 11. Post-Spike Actions

### If spike passes

1. Add Phase 4 task: **RecursiveQueryEngine** in `zen-search` with metadata-only planning + budget controls.
2. Add Phase 4 task: **ReferenceGraph** in `zen-search` (`symbol_refs`, `ref_edges`, category tagging, signature refs).
3. Add Phase 4 task: external reference scanner for cached dependency ecosystems (DataFusion-like cross-workspace evidence).
4. Add Phase 5 CLI flags: `--max-depth`, `--max-chunks`, `--max-bytes-per-chunk`, `--max-total-bytes`.
5. Add Phase 5 search output mode for categorized references and JSON summary payloads.
6. Tie doc-comment extraction + signature preservation into production search pipeline.

### If spike fails

1. Reduce scope to AST-only filtering first (no regex).
2. Re-run with smaller subset of Arrow crates as a baseline.
3. Revisit impl query patterns and trait/generic coverage.

---

## Implementation Notes (from spike)

- `ast-grep` works well for extraction; `tree-sitter` query API remains valuable for targeted structural checks and AST previews.
- Signature extraction should stay body-free; use a deterministic truncation strategy before `{`/`;` and normalize whitespace.
- Category naming settled as:
  - `same_module`
  - `other_module_same_crate`
  - `other_crate_workspace`
  - `external`
- External references are not inferred from AST joins; they are direct evidence lines from external crate source scans.

---

## 12. Cross-References

- Implementation plan: `zenith/docs/schema/07-implementation-plan.md`
- AST spike: `zenith/crates/zen-parser/src/spike_ast_grep.rs`
- RLM paper: `reference/llm-context-management/recursive_language_models.md`
- Arrow monorepo: `/Users/wrath/reference/rust/arrow-rs`
