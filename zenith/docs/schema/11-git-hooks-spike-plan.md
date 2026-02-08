# Zenith: Git Hooks & Session-Git Integration — Spike Plan

**Version**: 2026-02-08
**Status**: **DONE** — 22/22 tests pass. All decisions made. See [Spike Results](#spike-results) below.
**Purpose**: Validate git hooks implementation strategy (shell vs Rust), hook installation mechanism (core.hooksPath vs symlink vs chain), post-checkout/post-merge rebuild behavior (auto vs warn with performance data), and `gix` as the pure-Rust git library for detection, config, and session tagging
**Spike ID**: 0.13
**Crate**: zen-hooks (new crate, 10th in workspace)
**Blocks**: Tasks 5.18a-e (expanded from current 5.18), post-checkout/post-merge hooks, session-git integration

## Spike Results

**All 22 tests pass.** Key decisions:

| Decision | Choice | Evidence |
|----------|--------|----------|
| Hook implementation | **Approach C: Thin shell wrapper calling `znt hook <name>`** | Shell-only can't validate JSON (no jq by default). Rust with `serde_json` + `jsonschema` catches all edge cases: malformed JSON, BOM, conflict markers, missing required fields, invalid enum values (op type), wrong types. Schema is self-documenting and reusable. Wrapper gracefully skips when `znt` not in PATH. |
| Hook installation | **Strategy B: Symlink** for MVP | Coexists with most setups. Version-controlled in `.zenith/hooks/`. Detects existing hooks and refuses with guidance. Strategy A (`core.hooksPath`) available as future `--exclusive-hooks` option. |
| Post-checkout behavior | **Threshold-based auto-rebuild** | JSONL parse: 0.7ms/100 ops, 4.5ms/1000 ops, 22ms/5000 ops. Well under 500ms threshold. Full rebuild (with SQLite) is the bottleneck — measure in Phase 2. Default: auto-rebuild. |
| Git library | **`gix` adopted** for zen-hooks crate | All 9 operations validated. Pure Rust. Features: `max-performance-safe` + `index` + `blob-diff`. Isolated in zen-hooks — no compile impact on other crates. |
| Session-git | **Adopt lightweight session tags** | Branch/HEAD reading trivial. `edit_reference()` creates tags. `references().prefixed()` lists them. Rev-walk between tags works. **Gotcha**: `MustNotExist` doesn't reject duplicates in gix 0.70 — use `find_reference()` to check first. |
| CLI binary name | **`znt`** (not `zen`) | `zen` collides with zen-browser. All hook scripts reference `znt`. |

**Performance data (JSONL parse only, no SQLite):**

| Operations | Parse time | Ops/sec |
|-----------|-----------|---------|
| 100 | 0.7ms | ~140K |
| 1,000 | 4.5ms | ~220K |
| 5,000 | 22ms | ~218K |

**gix dependency:** `gix 0.70` with features `max-performance-safe`, `index`, `blob-diff`. Compiles in ~4s (incremental: ~1-2s). Isolated in zen-hooks crate.

**Gotchas discovered:**
- `gix` `BStr` → `String` conversion: use `.to_string()` on `Cow<BStr>` (not `.to_str()` which requires `ByteSlice` trait import)
- `gix` `config_snapshot_mut()` is **in-memory only** — must call `forget()` + `write_to()` to persist to `.git/config`
- `gix` `diff_tree_to_tree()` — `ChangeDetached::location()` is a method, not a field
- `gix` `PreviousValue::MustNotExist` does not reject duplicate refs in 0.70 — use `find_reference()` to check existence first
- Git hooks run with cwd = repo root, not `.git/hooks/` — use cwd-relative paths in hook scripts
- `zen` CLI name collides with zen-browser — renamed to `znt`
- `jsonschema` 0.28 `validator.validate()` returns `Err(ValidationError)` (single), use `validator.iter_errors()` for all errors
- `jsonschema` provides rich error messages out of the box: `"INVALID_OP" is not one of ["create","update",...]`, `[1,2,3] is not of type "object"` — much better than manual field checks
- `jq` is not installed by default on macOS or most Linux — cannot use for shell-only JSON validation

---

## Table of Contents

1. [Motivation](#1-motivation)
2. [Background & Prior Art](#2-background--prior-art)
3. [Open Questions](#3-open-questions)
4. [The Three Hook Implementation Approaches](#4-the-three-hook-implementation-approaches)
5. [The Three Installation Strategies](#5-the-three-installation-strategies)
6. [Session-Git Integration](#6-session-git-integration)
7. [Spike Tests](#7-spike-tests)
8. [Evaluation Criteria](#8-evaluation-criteria)
9. [Performance Thresholds](#9-performance-thresholds)
10. [What This Spike Does NOT Test](#10-what-this-spike-does-not-test)
11. [Success Criteria](#11-success-criteria)
12. [Post-Spike Actions](#12-post-spike-actions)

---

## 1. Motivation

The current implementation plan has task 5.18 as a single line: "Implement `zen init` .gitignore template and pre-commit hook." But the JSONL strategy doc (`10-git-jsonl-strategy.md`) describes three hooks and multiple open questions remain unaddressed:

| Gap | Why it matters |
|-----|---------------|
| Pre-commit hook is planned, post-checkout and post-merge are designed in `10-git-jsonl-strategy.md` but have no task IDs | After branch switch or merge, DB is stale — doesn't match JSONL trail on new branch |
| Hook implementation language undecided | Shell scripts can't reliably validate JSON without `jq` (not installed by default on macOS or many Linux distros). Rust validation via `zen hook` is robust but requires binary in PATH. |
| Hook installation mechanism unspecified | Users may have `core.hooksPath` set (husky, lefthook), existing hooks in `.git/hooks/`, or no git repo at all. Overwriting silently breaks their setup. |
| No `gix` spike | `10-git-jsonl-strategy.md` says "Git library: None" but we need reliable repo detection, config reading, staged-file detection, and JSONL-change detection. Shelling out to `git` CLI is fragile. |
| No rebuild performance data | Post-checkout auto-rebuild could block branch switches if trail is large. Need measured thresholds. |
| Session-git integration unexplored | Workflow tool plan uses git tags for session checkpoints. Current Zenith sessions are DB-only. Reading branch/HEAD and tagging sessions in git could improve cross-machine session visibility. |

### Why a New Crate

Git integration cross-cuts hooks, session management, and potentially future features (branch-aware whats-next, git-backed session restore). A dedicated `zen-hooks` crate:

- Isolates the `gix` dependency (heavy compile) from crates that don't need it
- Provides a clean API surface: `GitRepo` (detection/config), `HookManager` (install/uninstall), `SessionGit` (tags/branch)
- Can be made optional — if a project isn't a git repo, zen-hooks is simply not invoked

---

## 2. Background & Prior Art

### Current Zenith Decisions (from `10-git-jsonl-strategy.md`)

| Decision | Choice |
|----------|--------|
| Git operations | Not our responsibility — user/LLM handles `git add`/`commit` |
| What to `.gitignore` | `zenith.db`, `zenith.db-wal`, `zenith.db-shm`, `*.db-journal` |
| What to track in git | `.zenith/trail/*.jsonl`, `.zenith/hooks/`, `.zenith/config.toml` |
| Multi-agent model | Per-session JSONL files — concurrent-safe, merge-safe |
| JSONL trigger | Real-time append on every mutation |
| Planned hooks | Pre-commit (validate JSONL), post-checkout (rebuild DB if JSONL changed) |

### Hook Manager Landscape

| Tool | Language | Installation strategy | Coexistence behavior |
|------|----------|----------------------|---------------------|
| **husky** | JS | Sets `core.hooksPath` to `.husky/_` | Overwrites any existing `core.hooksPath`. Aggressive. |
| **lefthook** | Go | Copies scripts to `.git/hooks/` | Detects `core.hooksPath`, warns loudly, refuses to install unless `--force` |
| **pre-commit** | Python | Installs wrapper to `.git/hooks/pre-commit` | Refuses if `core.hooksPath` is set (local or global) |
| **cargo-husky** | Rust | Copies to `.git/hooks/` via `build.rs` | Overwrites existing hooks. No detection. |
| **prek** | Rust | Installs to `.git/hooks/` with thin wrappers | pre-commit reimplementation, compatible config format |

**Key insight**: There is no consensus on installation strategy. The safest tools (lefthook, pre-commit) detect conflicts and refuse. The most convenient (husky) just overwrites. We need to test all three strategies and pick based on evidence.

### Workflow Tool Git Integration (from `workflow-tool-plan/07-git-integration-strategy.md`)

The workflow tool plan uses `gix` for:
- Agent-specific branch isolation (over-engineered for Zenith's single-tool model)
- Per-agent JSONL files committed per-operation (Zenith commits are user's responsibility)
- Session management via branches and tags (relevant — we'll explore the tagging subset)
- Vector database rebuild from JSONL (same as Zenith's `zen rebuild`)
- Post-merge agent coordination triggers (simplified in Zenith to just rebuild)

We adapt the **useful subset**: `gix` for repo detection, config management, hook installation, and session tagging. We skip branch isolation and per-operation commits.

---

## 3. Open Questions

This spike must answer all of these:

| # | Question | Approaches to Test |
|---|----------|-------------------|
| 1 | Should hooks be shell scripts, Rust (`zen hook <name>`), or thin-shell-wrapping-Rust? | A (shell), B (Rust), C (thin wrapper) |
| 2 | How should hooks be installed? | `core.hooksPath`, symlink, chain-append |
| 3 | Should post-checkout auto-rebuild or warn? | Measure performance, apply thresholds |
| 4 | Is `gix` worth the dependency cost? | Measure compile time and binary size delta |
| 5 | Can `gix` read config, detect staged files, and diff trees reliably? | Direct testing |
| 6 | Can `gix` write config (`core.hooksPath`) and create tags? | Direct testing |
| 7 | Can we coexist with husky/lefthook/pre-commit? | Test detection and graceful behavior |
| 8 | Can `gix` support session-git integration (branch name, HEAD hash, tags)? | Direct testing |

---

## 4. The Three Hook Implementation Approaches

### Approach A: Pure Shell Scripts

Hooks are bash scripts generated by `zen init`, stored in `.zenith/hooks/`.

```bash
#!/bin/bash
# .zenith/hooks/pre-commit — validate JSONL trail files
STAGED=$(git diff --cached --name-only -- '.zenith/trail/*.jsonl')
for file in $STAGED; do
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        # Can't reliably validate JSON in pure bash without jq
        # Best effort: check line starts with { and ends with }
        case "$line" in
            \{*\}) ;;
            *) echo "ERROR: Invalid JSON in $file"; exit 1 ;;
        esac
    done < "$file"
done
```

**Pros**: Zero runtime dependency. Works even if `zen` isn't installed.
**Cons**: Cannot reliably validate JSON. No `jq` on most systems by default. The `{...}` check is trivially fooled by malformed JSON. Two codepaths (shell for hooks, Rust for everything else). Cannot do schema validation (checking required fields like `ts`, `ses`, `op`, `entity`).

### Approach B: `zen hook <name>` Subcommand

Hooks are 3-line shell wrappers that delegate to Rust:

```bash
#!/bin/bash
# .zenith/hooks/pre-commit
exec zen hook pre-commit "$@"
```

All validation logic lives in `zen hook pre-commit` (Rust):
- Parse each staged JSONL file with `serde_json`
- Validate schema: required fields present, entity types valid, timestamps parseable
- Report precise error locations (file:line)

**Pros**: Full `serde_json` validation. Schema checking. Testable with `cargo test`. Single codebase. Precise error messages.
**Cons**: Requires `zen` binary in PATH. If user hasn't installed `zen` globally (e.g., using `cargo run`), hooks fail.

### Approach C: Thin Wrapper with Graceful Fallback

```bash
#!/bin/bash
# .zenith/hooks/pre-commit
if command -v zen >/dev/null 2>&1; then
    exec zen hook pre-commit "$@"
else
    echo "zenith: 'zen' not in PATH — skipping JSONL validation"
    echo "zenith: install zen globally or run: cargo install --path crates/zen-cli"
    exit 0  # don't block the commit
fi
```

**Pros**: Works whether or not `zen` is installed. Clear guidance when `zen` is missing. Doesn't block development workflow.
**Cons**: When `zen` isn't in PATH, no validation happens at all. The "skip silently" behavior could lead to committing malformed JSONL.

**Note on `jq`**: Earlier designs considered falling back to `jq` or `python3` for JSON validation. Neither is reliably available: `jq` is not installed by default on macOS or many Linux distros; `python3` may not be present on minimal systems. The spike will confirm this limitation and document that **Rust validation via `zen hook` is the only reliable path**.

---

## 5. The Three Installation Strategies

### Strategy A: `core.hooksPath` (How Husky Does It)

```rust
// zen init sets:
repo.config_snapshot_mut()?.set_raw_value("core", None, "hooksPath", ".zenith/hooks")?;
```

- Hook scripts live in `.zenith/hooks/` (git-tracked, version-controlled)
- Git reads hooks from `.zenith/hooks/` instead of `.git/hooks/`
- No symlinks, no copying, no file conflicts
- Uninstall: `git config --unset core.hooksPath`

**Pros**: Cleanest approach. Hooks are version-controlled. No filesystem gymnastics. Every clone gets hooks automatically (once `core.hooksPath` is set).
**Cons**: **Exclusive** — replaces the default hooks path entirely. Any hooks the user had in `.git/hooks/` stop running. This is the most controversial aspect (see lefthook issue #1248 about husky conflicts).

### Strategy B: Symlink into `.git/hooks/`

```rust
// zen init creates:
std::os::unix::fs::symlink("../../.zenith/hooks/pre-commit", ".git/hooks/pre-commit")?;
```

- Hook scripts live in `.zenith/hooks/` (source of truth)
- Symlinks in `.git/hooks/` point to them
- Updating `.zenith/hooks/` source propagates without reinstall

**Pros**: Coexists with other hooks (only replaces hooks we create symlinks for). Source stays in `.zenith/hooks/`.
**Cons**: Symlink can break if directory structure changes. Some tools (cargo-husky) overwrite `.git/hooks/` contents. If user already has a `pre-commit` hook, symlink creation fails (or overwrites).

### Strategy C: Chain-Append to Existing Hooks

```bash
# If .git/hooks/pre-commit already exists:
# 1. Rename to .git/hooks/pre-commit.user
# 2. Install our hook that chains:
#!/bin/bash
# Run user's original hook first
if [ -x .git/hooks/pre-commit.user ]; then
    .git/hooks/pre-commit.user "$@" || exit $?
fi
# Then run zenith validation
zen hook pre-commit "$@"
```

**Pros**: Preserves user's existing hooks. Both hook chains run.
**Cons**: Most complex. Renaming files is fragile. If user reinstalls their hook manager, it overwrites our chain. Uninstall must restore the `.user` backup.

---

## 6. Session-Git Integration

Beyond hooks, `gix` enables lightweight session-git integration. Current Zenith sessions are DB-only — they have no git context. Adding git awareness enables:

| Feature | `gix` operation | Value |
|---------|----------------|-------|
| Stamp session with branch name | Read `HEAD` ref | Session knows what branch it was on |
| Stamp session with commit hash | Read `HEAD` object ID | Session knows what code state it started from |
| Tag session in git history | Create lightweight tag `zenith/ses-xxx` | `git log --decorate` shows session boundaries |
| List git-backed sessions | List tags matching `zenith/ses-*` | `zen session list` can show git-visible sessions |
| Show inter-session history | Rev-walk between two tags | "What happened between session A and session B?" |

This is **read-heavy, write-light**: 4 read operations, 1 write operation (tag creation). Tag creation is the only `gix` write operation beyond `core.hooksPath` config write.

**Design principle**: Session-git is additive. If the project isn't a git repo, sessions work exactly as they do now (DB-only). Git context is attached when available.

---

## 7. Spike Tests

**File**: `zenith/crates/zen-hooks/src/spike_git_hooks.rs`

### Part A: `gix` Repo Discovery & Config (3 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 1 | `spike_gix_discover_repo` | Use `gix::discover()` to find `.git` from a subdirectory of a temp repo. Verify: finds repo from nested dir, returns correct `.git` path. Handle: no-repo case returns clean error (not panic). Handle: bare repo detected. |
| 2 | `spike_gix_read_hooks_path` | Read `core.hooksPath` via `gix` config API. Test three states: (a) unset — should return default `.git/hooks`, (b) set to relative path `.husky/_`, (c) set to absolute path `/home/user/.git-hooks`. Verify we can distinguish all three and resolve relative paths correctly. |
| 3 | `spike_gix_detect_existing_hooks` | In a temp git repo, create `.git/hooks/pre-commit` (executable). Use `gix` + filesystem to detect it exists and is executable. Test: hook present + executable, hook present + not executable, hook absent. Measure: `gix` compile time delta and binary size delta (document in test output). |

### Part B: Hook Installation Strategies (4 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 4 | `spike_install_strategy_hookspath` | **Strategy A**: Use `gix` config write to set `core.hooksPath = .zenith/hooks`. Create a pre-commit script in `.zenith/hooks/`. Run `git commit` (empty, `--allow-empty`). Verify hook executes. Then unset `core.hooksPath`. Verify `.git/hooks/` hooks are active again. Evaluate: clean install, clean uninstall. |
| 5 | `spike_install_strategy_symlink` | **Strategy B**: Create symlink `.git/hooks/pre-commit` -> `../../.zenith/hooks/pre-commit`. Verify hook runs. Update source in `.zenith/hooks/`, verify updated hook runs without reinstall. Test: symlink target missing (graceful error). Test: existing hook at target path (refuse with message). |
| 6 | `spike_install_strategy_chain` | **Strategy C**: When `.git/hooks/pre-commit` already exists, rename to `pre-commit.user`, install our hook that chains both. Verify: user's hook runs first, then ours. Verify: if user's hook fails (exit 1), our hook doesn't run (chain respects exit codes). Test uninstall: restore `pre-commit.user` to `pre-commit`. |
| 7 | `spike_install_skip_no_git` | Call hook installation from a directory with no `.git`. Verify: returns Ok with a "not a git repository, skipping hooks" message. No error, no panic, no files created. |

### Part C: Pre-commit JSONL Validation (4 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 8 | `spike_precommit_shell_validation` | Write a pure-bash pre-commit script (no jq, no python). Test validation matrix against staged JSONL: (a) valid file — passes, (b) malformed JSON line — **cannot reliably detect** in pure bash (document this limitation), (c) empty file — passes, (d) trailing newline — passes, (e) BOM prefix — bash can't detect. Document: shell-only validation is insufficient. |
| 9 | `spike_precommit_rust_validation` | Implement `zen hook pre-commit` logic in Rust using `serde_json::from_str()` per line. Same matrix: (a) valid — passes, (b) malformed JSON — **detected with precise file:line error**, (c) empty file — passes, (d) trailing newline — passes, (e) BOM prefix — **detected and rejected**, (f) conflict markers (`<<<<<<<`) — **detected and rejected**, (g) required fields missing (`ts`, `ses`, `op`) — **detected with schema error**. Compare: Rust catches all edge cases shell misses. |
| 10 | `spike_precommit_staged_only` | Validate only *staged* JSONL files, not all files in `trail/`. Rust approach: use `gix` index API to read staged file paths matching `.zenith/trail/*.jsonl`. Verify: unstaged JSONL changes are ignored. Modified-but-not-staged files are ignored. Only `git add`'d files are validated. |
| 11 | `spike_precommit_wrapper` | Thin shell wrapper: if `zen` is in PATH, call `zen hook pre-commit`; else print guidance message and exit 0 (don't block commit). Test: (a) zen in PATH — full Rust validation runs, (b) zen not in PATH — prints "install zen for JSONL validation" and allows commit. Document: this is the recommended hook format. |

### Part D: Post-checkout & Post-merge (4 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 12 | `spike_postcheckout_detect_changes` | After simulated branch switch in temp repo, detect JSONL file changes between old HEAD and new HEAD. Use `gix` tree diff (compare trees of two commits). Verify: detects added JSONL, modified JSONL, deleted JSONL, ignores non-JSONL changes. Compare: `gix` tree diff vs shelling out to `git diff --name-only HEAD@{1} HEAD -- .zenith/trail/`. |
| 13 | `spike_postcheckout_auto_rebuild` | When JSONL changed on branch switch, trigger `zen rebuild` (simulated — replay JSONL to fresh DB). **Measure wall time** for: 100 operations, 1000 operations, 5000 operations. Record: wall time (ms), rebuilt DB size, FTS5 search works after rebuild. This test produces the data for the auto-vs-warn decision. |
| 14 | `spike_postcheckout_warn_only` | Alternative: detect changes, print "Zenith: JSONL trail changed on branch switch. Run `zen rebuild` to update database." and exit 0. Measure: near-zero delay. Compare: UX of auto-rebuild (seamless but slow) vs warn (instant but requires manual action). |
| 15 | `spike_postmerge_conflict_and_rebuild` | Simulate merge of two branches each with different JSONL trail files. (a) Clean merge (different session files — no conflict): verify rebuild produces DB with entities from both branches. (b) Conflict merge (same file modified — rare with per-session files but possible): verify conflict markers in JSONL are detected and reported. |

### Part E: Session-Git Integration (5 tests)

| # | Test | What It Validates |
|---|------|-------------------|
| 16 | `spike_session_read_branch` | Read current branch name via `gix`. Test: on `main`, on feature branch `feat/hooks`, on detached HEAD (should return None or detached indicator). Verify API is clean and doesn't panic on edge cases. |
| 17 | `spike_session_read_head` | Read HEAD commit hash via `gix`. Verify: returns full 40-char SHA. Verify: can also produce short (7-char) form. Test: empty repo with no commits (handle gracefully). |
| 18 | `spike_session_create_tag` | Create lightweight tag `zenith/ses-abc12345` at HEAD via `gix` refs API. Verify: tag exists in refs, points to correct commit. Test: create tag when tag already exists — should return clear error (not overwrite). Test: tag name with invalid characters — should reject. |
| 19 | `spike_session_list_tags` | List all refs matching `refs/tags/zenith/ses-*` via `gix`. Create 5 session tags, verify all 5 returned. Verify: non-zenith tags are excluded. Verify: returns tag name + commit hash for each. |
| 20 | `spike_session_commits_between_tags` | Create two session tags at different commits. Use `gix` rev-walk to list commits between them. Verify: correct commit count, correct chronological order, commit messages accessible. This validates we can answer "what happened between sessions." |

### Part F: Dependency Weight (1 test)

| # | Test | What It Validates |
|---|------|-------------------|
| 21 | `spike_gix_dependency_weight` | Document in test output: (a) `gix` features enabled vs disabled and their impact, (b) compile time of `zen-hooks` crate alone, (c) recommended minimal feature set for our use cases (discovery, config read/write, index read, tree diff, refs/tags, rev-walk). Note: binary size delta should be measured manually by comparing workspace build with and without zen-hooks. |

### Part G: Comparison & Decision (1 test)

| # | Test | What It Validates |
|---|------|-------------------|
| 22 | `spike_compare_all` | Print four comparison tables: **(1) Installation strategy**: `core.hooksPath` vs symlink vs chain — columns: coexistence with existing hooks, install complexity, uninstall complexity, version-controlled hooks, recommendation. **(2) Hook implementation**: shell vs Rust (`zen hook`) vs wrapper — columns: validation reliability (edge cases caught from tests 8-9), speed, testability, PATH dependency, recommendation. **(3) Post-checkout behavior**: auto-rebuild vs warn — columns: time at 100/1000/5000 ops (from test 13), UX impact, recommendation with threshold. **(4) `gix` verdict**: features used, dependency cost, alternatives (shell out to `git`), recommendation. |

**Total: 22 tests**

---

## 8. Evaluation Criteria

| Criterion | Weight | How We Measure |
|-----------|--------|---------------|
| JSONL validation reliability | **High** | Edge case matrix from tests 8-9: empty, BOM, partial write, conflict markers, schema validation, malformed JSON. Count of cases caught by each approach. |
| Installation safety | **High** | From tests 4-7: can we install without breaking existing hooks? Can we detect and respect `core.hooksPath`? Can we uninstall cleanly? |
| Rebuild performance | **High** | Wall time at 100, 1000, 5000 ops from test 13. Determines auto-rebuild vs warn-only threshold. |
| `gix` dependency cost | **Medium** | Compile time delta, recommended feature flags, API ergonomics from tests 1-3 and 21. |
| Session-git utility | **Medium** | From tests 16-20: is branch/HEAD/tag reading clean enough to be worth the integration? Does it add value to session metadata? |
| Developer UX | **Medium** | Auto-rebuild seamlessness vs warn-only discoverability. Hook installation "just works" vs "requires manual steps." |
| Testability | **Medium** | Can we test all hook behavior in `cargo test` using temp git repos? No real filesystem side effects outside tempdir. |
| Portability | **Low** | macOS + Linux. Windows is out of scope. Bash hooks assume `/bin/bash` or `/usr/bin/env bash`. |

---

## 9. Performance Thresholds

Based on git hook best practices (pre-commit should complete in < 10 seconds, post-checkout should feel instant):

| Rebuild time | Post-checkout behavior |
|-------------|----------------------|
| < 500ms | Auto-rebuild silently. User doesn't notice. |
| 500ms – 2s | Auto-rebuild with progress message: "Zenith: rebuilding database..." |
| 2s – 5s | Warn-only by default. Auto-rebuild available via `.zenith/config.toml` opt-in: `[hooks] auto_rebuild = true` |
| > 5s | Warn-only always. Auto-rebuild only via explicit `zen rebuild`. |

Pre-commit validation should always be < 1 second regardless of trail size (we only validate staged files, not the entire trail).

---

## 10. What This Spike Does NOT Test

- **Agent branch isolation** — workflow-tool feature, over-engineered for Zenith's single-tool CLI model
- **`gix` push/fetch/clone** — no remote operations. Zenith doesn't do git networking.
- **`gix` commit creation** — Zenith doesn't create commits. User/LLM responsibility.
- **JSONL compaction or archival** — future concern, not hooks-related
- **Windows platform** — out of scope. Bash hooks won't work on Windows without WSL/Git Bash.
- **CI/CD hook behavior** — hooks don't run in CI by default (`--no-verify` or no `.git/hooks/`)
- **Full session lifecycle** — we test git primitives (tag/branch/HEAD), not create/restore/checkpoint session workflow
- **Multi-agent branch coordination** — single-tool model, not multi-agent

---

## 11. Success Criteria

- All 22 tests compile and pass
- Clear recommendation for hook implementation: shell / Rust / wrapper — with evidence from validation matrix
- Clear recommendation for installation strategy: `core.hooksPath` / symlink / chain — with coexistence analysis
- Clear recommendation for post-checkout behavior: auto / warn — with performance data at three scales
- `gix` dependency evaluated: worth it or not, with compile time, feature flags, and API ergonomics
- Session-git integration evaluated: useful or not, with concrete API examples
- All decisions captured in spike module doc comments (following spike 0.2-0.12 pattern)

---

## 12. Post-Spike Actions

### Regardless of Outcome

1. Add `zen-hooks` crate to Cargo workspace (10th crate)
2. Add `gix` to workspace deps with minimal feature flags identified by spike
3. Update `07-implementation-plan.md`:
   - Add spike 0.13 to Phase 0 with results
   - Expand task 5.18 into 5.18a-e:
     - 5.18a: `.gitignore` template
     - 5.18b: Pre-commit hook (implementation chosen by spike)
     - 5.18c: Post-checkout hook (auto/warn chosen by spike)
     - 5.18d: Post-merge hook
     - 5.18e: Hook installation mechanism (strategy chosen by spike)
4. Update `10-git-jsonl-strategy.md`: add hook implementation decision, installation strategy, post-checkout behavior, coexistence guidance
5. Update `INDEX.md`: add zen-hooks to crate list, add document 11 to document map

### If Session-Git Tags Prove Useful

1. Update `07-implementation-plan.md`: add session-tag tasks to Phase 2 or Phase 5
2. Update `01-turso-data-model.md`: consider adding `git_branch` and `git_commit` columns to sessions table
3. Update `04-cli-api-design.md`: `zen session start` could accept `--tag` flag or auto-tag
4. Update `05-crate-designs.md`: add zen-hooks crate design with `SessionGit` module

### Risk Register Additions

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| User has existing git hooks (husky, lefthook, pre-commit) | Zenith hooks fail to install or overwrite user's hooks | Medium | Spike evaluates three installation strategies. Winner chosen for best coexistence. Detect `core.hooksPath` and existing hooks before installing. Support `--skip-hooks` flag. |
| `gix` adds significant compile time | Slower builds for all developers | Medium | Spike measures delta. `gix` is isolated in `zen-hooks` crate — only rebuilds when hooks code changes. Minimal feature flags reduce compile scope. |
| `zen rebuild` is too slow for post-checkout hook | Branch switches become sluggish | Low (< 5K ops) | Spike measures at 100/1000/5000 ops. Threshold-based decision: auto below threshold, warn above. Configurable via `.zenith/config.toml`. |
| `zen` binary not in PATH when hooks run | Hooks fail or skip validation silently | Medium | Wrapper approach (Approach C): graceful fallback with guidance message. Pre-commit skips validation rather than blocking commit. |

---

## Cross-References

- JSONL strategy (hooks design): [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md) §6-7
- Implementation plan (task 5.18): [07-implementation-plan.md](./07-implementation-plan.md)
- Workflow tool git integration: `workflow-tool-plan/07-git-integration-strategy.md` §6 (hooks system)
- Studies spike (structural precedent): [08-studies-spike-plan.md](./08-studies-spike-plan.md)
- JSONL spike (structural precedent): [10-git-jsonl-strategy.md](./10-git-jsonl-strategy.md) §9
- Reference: Beads Git layer — `reference/beads/architecture/git-layer.md`
- Reference: `gix` crate — `reference/tree-sitter/` (similar pure-Rust wrapping pattern)
