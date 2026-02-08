# Zenith: `zen grep` Feature Design

**Version**: 2026-02-08
**Status**: Design Document
**Purpose**: Two-engine hybrid grep — DuckDB for indexed package source, `grep`+`ignore` crates for local project files
**Spike**: `zen-search/src/spike_grep.rs` (spike 0.14)

---

## Table of Contents

1. [Overview](#1-overview)
2. [Design Decisions](#2-design-decisions)
3. [Storage: `source_files` Table](#3-storage-source_files-table)
4. [Indexing Pipeline Changes](#4-indexing-pipeline-changes)
5. [CLI Specification](#5-cli-specification)
6. [Output Format](#6-output-format)
7. [Symbol Correlation](#7-symbol-correlation)
8. [Architecture](#8-architecture)
9. [Walker Factory](#9-walker-factory)
10. [Cache Management](#10-cache-management)
11. [Dependencies](#11-dependencies)
12. [Implementation Tasks](#12-implementation-tasks)
13. [Risks](#13-risks)

---

## 1. Overview

`zen grep` adds regex/literal text search to Zenith. Two engines, two modes:

| Mode | Trigger | Engine | Data Source |
|------|---------|--------|-------------|
| **Package** | `--package <pkg>` or `--all-packages` | DuckDB `regexp_matches()` over stored source | `source_files` table in lake |
| **Local** | `<path...>` argument | `grep` + `ignore` crates (ripgrep's library) | Live filesystem |

**Why two engines**: Package source is already in DuckDB (compressed, no file sprawl). Local project files must be searched on the live filesystem. Each engine is optimal for its domain.

**Relationship to `rg`**: `zen grep` is additive, not a replacement. It provides structured JSON output for LLM consumption and Zenith-aware filtering (`.zenithignore`, test file skipping, symbol correlation). For human-interactive ad-hoc search, users should still use `rg` directly.

---

## 2. Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Package source storage | New `source_files` table in DuckDB lake | No file sprawl, FSST compressed (~2-3x), single `.duckdb` file, correlates with `api_symbols` via SQL JOIN |
| Package grep engine | DuckDB fetch + Rust `regex` crate for line matching | DuckDB handles compressed storage + filtering; Rust regex handles line splitting + matching (faster than doing it all in SQL) |
| Local grep engine | `grep` 0.4 + `ignore` 0.4 crates | ripgrep-quality speed (~10-50ms), `.gitignore` aware, mature ecosystem |
| Default behavior (no flags) | **Error** — must provide `--package`, `--all-packages`, or `[path...]` | `rg` already handles ad-hoc local search well; `zen grep` should be explicit about scope |
| `--all-packages` | Supported | User option to search across all cached packages |
| Test file caching | Cache all source, filter at grep time | Faster `zen install` (no test-detection overhead at cache time), no re-install needed to grep tests later, `ignore` crate's `filter_entry` skips files before any I/O |
| Symbol correlation | Package mode only, via batch query + binary search | Local project files aren't indexed in `api_symbols` |
| File retention on disk | **None** — source lives inside DuckDB | Avoids thousands of files in `.zenith/cache/sources/`, cleaner than raw file retention |

---

## 3. Storage: `source_files` Table

Added to the DuckDB lake alongside `api_symbols`, `doc_chunks`, and `indexed_packages`.

```sql
CREATE TABLE source_files (
    ecosystem TEXT NOT NULL,
    package TEXT NOT NULL,
    version TEXT NOT NULL,
    file_path TEXT NOT NULL,      -- relative path within repo (e.g., "src/runtime/blocking/pool.rs")
    content TEXT NOT NULL,         -- full file content, unmodified
    language TEXT,                 -- detected language ("rust", "python", "typescript", etc.)
    size_bytes INTEGER,            -- original file size in bytes
    line_count INTEGER,            -- pre-computed line count (for stats)
    PRIMARY KEY (ecosystem, package, version, file_path)
);

CREATE INDEX idx_source_pkg ON source_files(ecosystem, package, version);
CREATE INDEX idx_source_lang ON source_files(ecosystem, package, version, language);
```

**Size estimate** (10 typical packages, ~50 MB raw source):

| Storage | Size |
|---------|------|
| Raw files on disk | ~50 MB |
| DuckDB native (FSST auto-compressed) | ~20-25 MB |
| Parquet + zstd | ~10-15 MB |

DuckDB's FSST compression is particularly effective on source code due to highly repetitive keywords (`fn`, `pub`, `let`, `struct`, `return`, indentation patterns).

**New column on `indexed_packages`**: `source_cached BOOLEAN DEFAULT FALSE` — tracks whether source files were stored for this package. Enables graceful degradation for packages indexed before the grep feature.

---

## 4. Indexing Pipeline Changes

Current pipeline step 8 (`02-ducklake-data-model.md:399`): `rm -rf /tmp/zenith-index/<pkg>`

**New step 6.5** (between "Write to DuckLake" and "Update Turso"):

```
6.5 Store Source Files
    For each source file walked in step 2 (already in memory from step 3):
      INSERT INTO source_files (ecosystem, package, version, file_path, content, language, size_bytes, line_count)
    UPDATE indexed_packages SET source_cached = TRUE
```

This adds zero I/O — the file content is already in memory from the parsing step. We just also write it to DuckDB before discarding.

Step 8 remains: `rm -rf /tmp/zenith-index/<pkg>` — source is in DuckDB, temp clone is still deleted.

---

## 5. CLI Specification

```
zen grep <pattern> [path...] [flags]

REQUIRED: one of --package, --all-packages, or [path...] must be provided.
```

**Modes:**

```bash
zen grep <pattern> --package <pkg>        # Search one indexed package's source
zen grep <pattern> -P tokio -P serde      # Search multiple packages
zen grep <pattern> --all-packages         # Search all indexed packages
zen grep <pattern> <path...>              # Search local project files
```

**Flags:**

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--package` | `-P` | Search cached source for this package (repeatable) | (none) |
| `--ecosystem` | `-e` | Ecosystem filter (with `--package`) | auto-detect |
| `--all-packages` | | Search all indexed packages | `false` |
| `--fixed-strings` | `-F` | Treat pattern as literal, not regex | `false` |
| `--ignore-case` | `-i` | Case-insensitive matching | `false` |
| `--smart-case` | `-S` | Auto case-insensitive if pattern is all lowercase | `true` |
| `--word-regexp` | `-w` | Whole word matching | `false` |
| `--context` | `-C` | Lines of context around matches | `2` |
| `--before-context` | `-B` | Lines before match | (uses `-C`) |
| `--after-context` | `-A` | Lines after match | (uses `-C`) |
| `--include` | | File glob to include (e.g., `"*.rs"`) | (all files) |
| `--exclude` | | File glob to exclude | (none) |
| `--max-count` | `-m` | Max matches per file | (none) |
| `--count` | `-c` | Only show match counts per file | `false` |
| `--files-with-matches` | `-l` | Only show filenames with matches | `false` |
| `--skip-tests` | | Skip test files/dirs | `false` |
| `--no-symbols` | | Skip symbol correlation (package mode only) | `false` |
| `--multiline` | `-U` | Match across line boundaries | `false` |

**Validation rules:**

- Error if no scope provided (no `--package`, `--all-packages`, or `[path...]`)
- Error if `--package`/`--all-packages` and `[path...]` both provided
- `--no-symbols` ignored in local mode
- `--skip-tests` applies to both modes (filters files by name in package mode, filters walked entries in local mode)

---

## 6. Output Format

### Standard matches (JSON, default)

```json
{
  "matches": [
    {
      "path": "tokio/src/runtime/blocking/pool.rs",
      "line_number": 142,
      "text": "    pub(crate) fn spawn_blocking(&self, func: ...) {",
      "context_before": ["    /// Spawns a blocking task.", "    ///"],
      "context_after": ["        let (task, handle) = ..."],
      "symbol": {
        "id": "abc123",
        "kind": "function",
        "name": "spawn_blocking",
        "signature": "pub(crate) fn spawn_blocking<F, R>(&self, func: F) -> JoinHandle<R>"
      }
    },
    {
      "path": "tokio/src/runtime/blocking/pool.rs",
      "line_number": 87,
      "text": "    // spawn_blocking is called from the runtime handle",
      "context_before": [],
      "context_after": [],
      "symbol": null
    }
  ],
  "stats": {
    "files_searched": 284,
    "files_matched": 12,
    "matches_found": 37,
    "matches_with_symbol": 28,
    "elapsed_ms": 45
  }
}
```

`symbol` is `null` when the match falls outside any indexed symbol's line range, when `--no-symbols` is passed, or in local mode.

### Count mode (`-c`)

```json
{
  "counts": [
    {"path": "src/runtime/blocking/pool.rs", "count": 5},
    {"path": "src/task/spawn.rs", "count": 3}
  ],
  "stats": {"files_searched": 284, "files_matched": 8, "total_matches": 37, "elapsed_ms": 32}
}
```

### Files mode (`-l`)

```json
{
  "files": ["src/runtime/blocking/pool.rs", "src/task/spawn.rs"],
  "stats": {"files_searched": 284, "files_matched": 8, "elapsed_ms": 18}
}
```

---

## 7. Symbol Correlation

In package mode, grep matches are correlated with `api_symbols` entries by file path and line range.

**New index on `api_symbols`:**

```sql
CREATE INDEX idx_symbols_file_lines
    ON api_symbols(ecosystem, package, version, file_path, line_start, line_end);
```

**Algorithm** (per-file batch, not per-match):

1. Grep produces matches grouped by file path
2. For each file with matches, fetch all symbols:
   ```sql
   SELECT id, kind, name, signature, line_start, line_end
   FROM api_symbols
   WHERE ecosystem = ? AND package = ? AND version = ? AND file_path = ?
   ORDER BY line_start;
   ```
3. For each match at line N, binary search: find symbol where `line_start <= N <= line_end`
4. Attach `SymbolRef` or `null`

**Performance**: One DuckDB query per matched file (~5-20ms total for typical grep output).

---

## 8. Architecture

### Package Mode Flow

```
zen grep "pattern" --package tokio
  │
  ├── Build regex from pattern + flags (case_insensitive, word, etc.)
  ├── Query DuckDB: SELECT file_path, content FROM source_files WHERE ...
  │     └── Filter by ecosystem, package, version, language (from --include glob)
  ├── For each file (in Rust, not SQL):
  │     ├── Split content by newlines
  │     ├── Apply regex to each line
  │     ├── Collect matches + context lines
  │     └── Apply --max-count, --skip-tests filters
  ├── Correlate with api_symbols (batch per-file, binary search)
  └── Return GrepResult JSON
```

### Local Mode Flow

```
zen grep "pattern" src/
  │
  ├── Build RegexMatcher (grep-regex crate)
  ├── Build WalkBuilder (ignore crate):
  │     ├── .gitignore respected (default)
  │     ├── .zenith/ skipped (override)
  │     ├── --include/--exclude (overrides)
  │     ├── --skip-tests (filter_entry via zen_parser::is_test_*)
  │     └── .zenithignore (custom ignore filename)
  ├── Walk files, search each with grep::searcher::Searcher
  ├── Collect matches via custom Sink → GrepMatch structs
  └── Return GrepResult JSON (symbol: null for all)
```

### Module Layout

```
zen-search/src/
├── lib.rs            # Re-export SearchEngine + GrepEngine
├── vector.rs         # (existing planned) Vector search
├── fts.rs            # (existing planned) FTS5 search
├── hybrid.rs         # (existing planned) Hybrid search
├── grep.rs           # NEW: GrepEngine (package mode + local mode)
└── walk.rs           # NEW: File walker factory (ignore crate integration)

zen-cli/src/commands/
├── grep.rs           # NEW: CLI handler for zen grep
├── cache.rs          # NEW: CLI handler for zen cache
└── ...existing...
```

### Key Types

```rust
// zen-search/src/grep.rs

pub struct GrepEngine {
    lake: Option<ZenLake>,  // For package mode (source_files + symbol correlation)
}

pub struct GrepOptions {
    pub case_insensitive: bool,
    pub smart_case: bool,
    pub fixed_strings: bool,
    pub word_regexp: bool,
    pub multiline: bool,
    pub context_before: u32,
    pub context_after: u32,
    pub include_glob: Option<String>,
    pub exclude_glob: Option<String>,
    pub max_count: Option<u32>,
    pub skip_tests: bool,
    pub no_symbols: bool,
}

pub struct GrepResult {
    pub matches: Vec<GrepMatch>,
    pub stats: GrepStats,
}

pub struct GrepMatch {
    pub path: String,
    pub line_number: u64,
    pub text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub symbol: Option<SymbolRef>,
}

pub struct SymbolRef {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub signature: String,
}

pub struct GrepStats {
    pub files_searched: u64,
    pub files_matched: u64,
    pub matches_found: u64,
    pub matches_with_symbol: u64,
    pub elapsed_ms: u64,
}
```

---

## 9. Walker Factory

Shared between `zen grep` local mode and the indexing pipeline (Phase 3, task 3.14).

```rust
// zen-search/src/walk.rs

pub enum WalkMode {
    /// Local project: respect .gitignore, skip .zenith/, support .zenithignore
    LocalProject,
    /// Raw: no filters (for internal use)
    Raw,
}

pub fn build_walker(
    root: &Path,
    mode: WalkMode,
    skip_tests: bool,
    include_glob: Option<&str>,
    exclude_glob: Option<&str>,
) -> ignore::Walk { ... }
```

**`LocalProject` mode:**
- `.gitignore` respected (default `WalkBuilder` behavior)
- `.zenith/` always skipped (via override, highest priority)
- `--include`/`--exclude` applied as overrides
- `--skip-tests` uses `filter_entry` calling `zen_parser::is_test_dir()`/`is_test_file()`
- `.zenithignore` files auto-discovered per-directory

---

## 10. Cache Management

New `zen cache` command for managing stored source in DuckDB:

```
zen cache
├── list                    # Show packages with cached source + sizes
├── clean                   # Remove all cached source
├── clean <package>         # Remove one package's cached source
└── stats                   # Total cache size, package count
```

**Implementation**: SQL operations on `source_files` table:

```sql
-- list
SELECT ecosystem, package, version,
       COUNT(*) as file_count, SUM(size_bytes) as total_bytes
FROM source_files GROUP BY ecosystem, package, version;

-- clean <package>
DELETE FROM source_files WHERE ecosystem = ? AND package = ?;
UPDATE indexed_packages SET source_cached = FALSE WHERE ecosystem = ? AND name = ?;

-- clean (all)
DELETE FROM source_files;
UPDATE indexed_packages SET source_cached = FALSE;

-- stats
SELECT COUNT(DISTINCT (ecosystem, package, version)) as packages,
       COUNT(*) as files, SUM(size_bytes) as total_bytes
FROM source_files;
```

---

## 11. Dependencies

**New workspace dependencies:**

```toml
# Grep (ripgrep library — for local project search)
grep = "0.4"
ignore = "0.4"
```

**Crate-level additions:**

| Crate | Gets | Why |
|-------|------|-----|
| `zen-search` | `grep`, `ignore`, `duckdb`, `regex` | Local mode (grep+ignore), package mode (duckdb fetch + regex match) |
| `zen-lake` | (no new deps) | `source_files` table uses existing `duckdb` |
| `zen-cli` | (no new deps) | Delegates to `zen-search` |

---

## 12. Implementation Tasks

| ID | Task | Crate | Phase | Blocks |
|----|------|-------|-------|--------|
| 0.14 | **Spike**: Validate `grep` crate, `ignore` crate, DuckDB `source_files` + `regexp_matches`, symbol correlation | zen-search | 0 | All below |
| 3.16 | Add `source_files` table to DuckDB schema, add `source_cached` to `indexed_packages` | zen-lake | 3 | 3.17 |
| 3.17 | Store source file contents during indexing pipeline (step 6.5) | zen-lake | 3 | 4.10 |
| 3.18 | Implement `walk.rs` walker factory (`WalkMode::LocalProject`, `Raw`) | zen-search | 3 | 4.10, 3.14 |
| 4.10 | Implement `GrepEngine::grep_package()` — DuckDB fetch + Rust regex + symbol correlation | zen-search | 4 | 5.19 |
| 4.11 | Implement `GrepEngine::grep_local()` — `grep` + `ignore` crates, custom `Sink` | zen-search | 4 | 5.19 |
| 4.12 | Add `idx_symbols_file_lines` index to `api_symbols` | zen-lake | 4 | 4.10 |
| 5.19 | Implement `zen grep` CLI command (both modes, all flags) | zen-cli | 5 | Done |
| 5.20 | Implement `zen cache` CLI command (list, clean, stats) | zen-cli | 5 | Done |

**Critical path**: 0.14 → 3.16 → 3.17 → 4.10 → 5.19

---

## 13. Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| DuckDB `string_split`+`unnest` too slow for large packages | Grep latency >500ms | Low | Fetch content from DuckDB, do line splitting + regex in Rust. DuckDB is just compressed storage. |
| `source_files` doubles lake file size | Disk concern for users with many indexed packages | Medium | `zen cache clean` command. Can add `--no-cache-source` flag to `zen install` if needed. |
| Regex semantics differ between DuckDB (RE2) and grep crate (Rust `regex`) | Pattern works in one mode but not the other | Low | Both are RE2-compatible (linear time, no backtracking). Document minor differences. |
| Package indexed before grep feature has no `source_files` data | `zen grep --package <pkg>` fails | Medium | Check `source_cached` flag, return clear error with `zen install <pkg> --force` guidance. |
| `grep` crate API changes | Breaks local mode | Very low | v0.4 is stable, BurntSushi actively maintains it. |

---

## Cross-References

- DuckDB data model: [02-ducklake-data-model.md](./02-ducklake-data-model.md)
- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md)
- Crate designs: [05-crate-designs.md](./05-crate-designs.md)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md)
