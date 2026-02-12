# `types.rs` Refactor Plan

This plan captures the agreed approach for splitting `src/types.rs` in small, low-risk steps across multiple sessions.

## Goals

- Split `src/types.rs` into modules for maintainability.
- Keep public API stable for all extractors.
- Avoid behavior changes during the mechanical split.
- Run formatting, lint, and tests as hard gates.

## Constraints

- Preserve these imports everywhere:
  - `crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility, DocSections}`
- Keep `SymbolMetadata` shape unchanged (fields, names, types, serde derives/attrs).
- Keep `SymbolKind` and `Visibility` variants and `Display` output unchanged.
- Do not modify extractor behavior in this refactor unless a compile fix is unavoidable.

## Current Workspace Context

There are ongoing local edits in:

- `src/extractors/rust.rs`
- `src/extractors/python.rs`
- `src/extractors/helpers.rs`
- fixture files under `tests/fixtures/`

Because of that, this refactor should remain structural and API-preserving to minimize merge risk.

## Preflight Baseline (Before Session 1)

Run the full quality gates once before touching files so failures can be attributed correctly:

1. `cargo fmt --all --check`
2. `cargo clippy -p zen-parser --all-targets --all-features -- -D warnings`
3. `cargo test -p zen-parser`

If baseline already fails, capture and keep that record separate from refactor-introduced issues.

## Target Layout (Phase A)

Replace `src/types.rs` with:

- `src/types/mod.rs`
- `src/types/parsed_item.rs`
- `src/types/symbol_kind.rs`
- `src/types/visibility.rs`
- `src/types/doc_sections.rs`
- `src/types/symbol_metadata/mod.rs`
- `src/types/symbol_metadata/bash.rs`
- `src/types/symbol_metadata/c.rs`
- `src/types/symbol_metadata/cpp.rs`
- `src/types/symbol_metadata/go.rs`
- `src/types/symbol_metadata/elixir.rs`
- `src/types/symbol_metadata/javascript.rs`
- `src/types/symbol_metadata/python.rs`
- `src/types/symbol_metadata/rust.rs`
- `src/types/symbol_metadata/typescript.rs`
- `src/types/symbol_metadata/tsx.rs`
- `src/types/symbol_metadata/html.rs`
- `src/types/symbol_metadata/css.rs`

Important: Rust module loading does not allow both `src/types.rs` and `src/types/mod.rs` simultaneously for the same module. The cutover must remove `src/types.rs` in the same refactor step that introduces `src/types/mod.rs`.

## Mechanical Move Map

From current `src/types.rs`:

- `ParsedItem` -> `src/types/parsed_item.rs`
- `SymbolKind` + `impl Display` -> `src/types/symbol_kind.rs`
- `Visibility` + `impl Display` -> `src/types/visibility.rs`
- `DocSections` -> `src/types/doc_sections.rs`
- `SymbolMetadata` -> `src/types/symbol_metadata/mod.rs`

`src/types/mod.rs` must re-export:

- `ParsedItem`
- `SymbolKind`
- `Visibility`
- `SymbolMetadata`
- `DocSections`

Delete `src/types.rs` only after all new modules compile.

## Session Breakdown

### Session 1: Mechanical Split

1. Create only core split files and module declarations:
   - `src/types/mod.rs`
   - `src/types/parsed_item.rs`
   - `src/types/symbol_kind.rs`
   - `src/types/visibility.rs`
   - `src/types/doc_sections.rs`
   - `src/types/symbol_metadata/mod.rs`
2. Move existing definitions 1:1.
3. Add `types/mod.rs` re-exports.
4. Remove old `src/types.rs`.
5. Fix compile/import fallout only.
6. Do not change extractor behavior; avoid edits under `src/extractors/*` unless strictly required for compile-path fixes.

Deliverable: no behavioral changes, same public API.

### Session 2: Language Metadata Modules (Additive)

1. Add per-language metadata files:
   - `src/types/symbol_metadata/bash.rs`
   - `src/types/symbol_metadata/c.rs`
   - `src/types/symbol_metadata/cpp.rs`
   - `src/types/symbol_metadata/go.rs`
   - `src/types/symbol_metadata/elixir.rs`
   - `src/types/symbol_metadata/javascript.rs`
   - `src/types/symbol_metadata/python.rs`
   - `src/types/symbol_metadata/rust.rs`
   - `src/types/symbol_metadata/typescript.rs`
   - `src/types/symbol_metadata/tsx.rs`
   - `src/types/symbol_metadata/html.rs`
   - `src/types/symbol_metadata/css.rs`
2. Wire `pub mod ...` declarations from `src/types/symbol_metadata/mod.rs`.
3. Seed modules with additive helpers/traits only.
4. Keep helpers optional and non-breaking.
5. Avoid changing extractor call sites unless intentionally adopting helpers.

Deliverable: better organization, still no schema changes.

### Session 3 (Later): Deep Split (Option 2)

If needed, per-language module directories:

- e.g. `src/types/symbol_metadata/rust/mod.rs`, `helpers.rs`, etc.

Do incrementally per language that grows.

## Validation Gates (Required Every Session)

Run in this order:

1. `cargo fmt --all --check`
2. `cargo clippy -p zen-parser --all-targets --all-features -- -D warnings`
3. `cargo test -p zen-parser`

## API/Serde Compatibility Checks

Add a small compatibility test (or snapshot assertion) verifying serialized shape remains stable for:

- `ParsedItem`
- `SymbolMetadata`
- `DocSections`

At minimum, ensure expected field names and enum variant serialization are unchanged.

## Done Criteria (Phase A)

- `src/types.rs` replaced by module tree.
- All existing `crate::types::{...}` imports continue to work.
- No public type/schema changes.
- `fmt`, `clippy`, and `test` pass.

## Notes for Next Refactor

After this lands cleanly, proceed with `extractors/bash.rs` split in a separate change set.
