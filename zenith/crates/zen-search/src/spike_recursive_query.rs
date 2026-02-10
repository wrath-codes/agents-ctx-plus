//! # Spike 0.21: Recursive Context Query (RLM-style) on Arrow Monorepo
//!
//! Validates an RLM-style query loop over a large real codebase using:
//! - metadata-only root planning
//! - AST-based symbol extraction (ast-grep)
//! - tree-sitter query fallback for impl patterns
//! - budgeted recursive processing and deterministic output assembly

#[cfg(test)]
mod tests {
    use ast_grep_core::matcher::KindMatcher;
    use ast_grep_language::{LanguageExt, SupportLang};
    use duckdb::{params, Connection};
    use ignore::WalkBuilder;
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tree_sitter::StreamingIterator;

    const ARROW_ROOT: &str = "/Users/wrath/reference/rust/arrow-rs";
    const CARGO_REGISTRY_SRC: &str = "/Users/wrath/.cargo/registry/src";

    #[derive(Debug, Clone)]
    struct FileMeta {
        path: PathBuf,
        bytes: usize,
        lines: usize,
    }

    #[derive(Debug, Clone)]
    struct SymbolHit {
        file_path: PathBuf,
        kind: String,
        name: String,
        line_start: usize,
        line_end: usize,
        signature: String,
        doc: String,
    }

    #[derive(Debug, Clone, Copy)]
    struct Budgets {
        max_depth: usize,
        max_chunks: usize,
        max_bytes_per_chunk: usize,
        max_total_bytes: usize,
    }

    fn require_arrow_root() -> PathBuf {
        let root = PathBuf::from(ARROW_ROOT);
        assert!(
            root.exists(),
            "Arrow monorepo not found at {}",
            root.display()
        );
        root
    }

    fn rust_files(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(false)
            .git_exclude(false)
            .build();

        for entry in walker.filter_map(Result::ok) {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path.to_path_buf());
            }
        }
        files
    }

    fn collect_metadata(root: &Path) -> Vec<FileMeta> {
        rust_files(root)
            .into_iter()
            .filter_map(|path| {
                let Ok(content) = fs::read_to_string(&path) else {
                    return None;
                };
                Some(FileMeta {
                    path,
                    bytes: content.len(),
                    lines: content.lines().count(),
                })
            })
            .collect()
    }

    fn leading_doc_comment(source: &str, zero_based_start_line: usize) -> String {
        let lines: Vec<&str> = source.lines().collect();
        if lines.is_empty() || zero_based_start_line >= lines.len() {
            return String::new();
        }

        let mut docs = Vec::new();
        let mut idx = zero_based_start_line;
        loop {
            if idx >= lines.len() {
                break;
            }
            let line = lines[idx].trim_start();
            if line.starts_with("///") {
                docs.push(line.trim_start_matches("///").trim().to_string());
            } else if line.starts_with("//!") {
                docs.push(line.trim_start_matches("//!").trim().to_string());
            } else if line.is_empty() {
                if docs.is_empty() {
                    // Skip one immediate blank line between docs and item.
                } else {
                    break;
                }
            } else {
                if docs.is_empty() && idx > 0 {
                    idx -= 1;
                    continue;
                }
                break;
            }

            if idx == 0 {
                break;
            }
            idx -= 1;
        }

        docs.reverse();
        docs.join("\n")
    }

    fn extract_hits_from_file(path: &Path, keywords: &[&str]) -> Vec<SymbolHit> {
        let Ok(source) = fs::read_to_string(path) else {
            return Vec::new();
        };
        let root = SupportLang::Rust.ast_grep(&source);
        let node = root.root();

        let kinds = [
            ("function_item", "function"),
            ("struct_item", "struct"),
            ("enum_item", "enum"),
            ("trait_item", "trait"),
        ];

        let mut out = Vec::new();
        for (kind_name, label) in kinds {
            let matcher = KindMatcher::new(kind_name, SupportLang::Rust);
            for m in node.find_all(matcher) {
                let name = m
                    .field("name")
                    .map(|n| n.text().to_string())
                    .unwrap_or_else(|| "<anon>".to_string());
                let start = m.start_pos().line();
                let end = m.end_pos().line();
                let signature = extract_signature_from_node_text(&m.text());
                let doc = leading_doc_comment(&source, start);
                if doc.is_empty() {
                    continue;
                }
                let lowered = doc.to_lowercase();
                let has_keyword = keywords.iter().any(|k| lowered.contains(&k.to_lowercase()));
                if !has_keyword {
                    continue;
                }

                out.push(SymbolHit {
                    file_path: path.to_path_buf(),
                    kind: label.to_string(),
                    name,
                    line_start: start + 1,
                    line_end: end + 1,
                    signature,
                    doc,
                });
            }
        }
        out
    }

    fn extract_signature_from_node_text(node_text: &str) -> String {
        let trimmed = node_text.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        let mut best = trimmed.len();
        for sep in [" {", "{", ";"] {
            if let Some(idx) = trimmed.find(sep) {
                best = best.min(idx);
            }
        }

        let sig = trimmed[..best].trim();
        sig.replace('\n', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn hit_ref_id(hit: &SymbolHit) -> String {
        format!(
            "{}::{}::{}::{}",
            hit.file_path.to_string_lossy(),
            hit.kind,
            hit.name,
            hit.line_start
        )
    }

    fn build_signature_index(hits: &[SymbolHit]) -> BTreeMap<String, String> {
        let mut idx = BTreeMap::new();
        for h in hits {
            idx.insert(hit_ref_id(h), h.signature.clone());
        }
        idx
    }

    fn signature_for_hit(h: &SymbolHit, idx: &BTreeMap<String, String>) -> Option<String> {
        idx.get(&hit_ref_id(h)).cloned()
    }

    fn shared_themes(a_doc: &str, b_doc: &str) -> Vec<&'static str> {
        let a = a_doc.to_lowercase();
        let b = b_doc.to_lowercase();
        ["invariant", "safety", "panic"]
            .iter()
            .filter_map(|k| (a.contains(k) && b.contains(k)).then_some(*k))
            .collect()
    }

    fn build_internal_edges(
        type_hits: &[SymbolHit],
        fn_hits: &[SymbolHit],
        max_type: usize,
        max_fn: usize,
    ) -> Vec<(String, String, String, String)> {
        let mut edges = Vec::new();
        for t in type_hits.iter().take(max_type) {
            for f in fn_hits.iter().take(max_fn) {
                let themes = shared_themes(&t.doc, &f.doc);
                if themes.is_empty() {
                    continue;
                }
                let category = categorize_reference(&t.file_path, &f.file_path).to_string();
                let evidence = format!("shared_themes={}", themes.join(","));
                edges.push((hit_ref_id(t), hit_ref_id(f), category, evidence));
            }
        }
        edges
    }

    fn setup_ref_graph_db() -> Connection {
        let conn = Connection::open_in_memory().expect("duckdb in-memory should open");
        conn.execute_batch(
            "CREATE TABLE symbol_refs (
                ref_id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                line_start INTEGER NOT NULL,
                line_end INTEGER NOT NULL,
                signature TEXT NOT NULL,
                doc TEXT NOT NULL
            );
            CREATE TABLE ref_edges (
                edge_id TEXT PRIMARY KEY,
                source_ref_id TEXT NOT NULL,
                target_ref_id TEXT NOT NULL,
                category TEXT NOT NULL,
                evidence TEXT,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX idx_ref_edges_source ON ref_edges(source_ref_id);
            CREATE INDEX idx_ref_edges_target ON ref_edges(target_ref_id);
            CREATE INDEX idx_ref_edges_category ON ref_edges(category);",
        )
        .expect("ref graph schema should create");
        conn
    }

    fn signature_lookup_db(conn: &Connection, ref_id: &str) -> Option<String> {
        conn.query_row(
            "SELECT signature FROM symbol_refs WHERE ref_id = ?",
            params![ref_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
    }

    fn baseline_impl_query() -> &'static str {
        "(impl_item type: (type_identifier) @name) @impl"
    }

    fn extended_impl_query() -> &'static str {
        r#"
        (impl_item
          trait: (type_identifier) @trait
          type: (type_identifier) @name) @impl

        (impl_item
          trait: (scoped_type_identifier) @trait
          type: (type_identifier) @name) @impl

        (impl_item
          type: (generic_type
                  type: (type_identifier) @name)) @impl

        (impl_item
          type: (scoped_type_identifier) @name) @impl

        (impl_item
          trait: (type_identifier) @trait
          type: (generic_type
                  type: (type_identifier) @name)) @impl

        (impl_item
          trait: (scoped_type_identifier) @trait
          type: (generic_type
                  type: (type_identifier) @name)) @impl
        "#
    }

    fn run_impl_query(source: &str, query_src: &str) -> usize {
        let mut parser = tree_sitter::Parser::new();
        let lang = SupportLang::Rust.get_ts_language();
        parser
            .set_language(&lang)
            .expect("set Rust ts language should succeed");
        let tree = parser
            .parse(source.as_bytes(), None)
            .expect("parse should succeed");
        let root = tree.root_node();

        let query = tree_sitter::Query::new(&lang, query_src).expect("query should compile");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        let mut count = 0usize;
        while matches.next().is_some() {
            count += 1;
        }
        count
    }

    fn ast_tree_sexp_preview(path: &Path, max_chars: usize) -> String {
        let source = fs::read_to_string(path).expect("source file should read");
        let mut parser = tree_sitter::Parser::new();
        let lang = SupportLang::Rust.get_ts_language();
        parser
            .set_language(&lang)
            .expect("set Rust ts language should succeed");
        let tree = parser
            .parse(source.as_bytes(), None)
            .expect("parse should succeed");
        let sexp = tree.root_node().to_sexp();
        if sexp.len() <= max_chars {
            return sexp;
        }
        format!("{}...", &sexp[..max_chars])
    }

    fn build_linear_summary(
        root: &Path,
        budgets: Budgets,
        keywords: &[&str],
    ) -> (Vec<SymbolHit>, usize) {
        let all_meta = collect_metadata(root);
        let mut selected_files = Vec::new();
        let mut consumed = 0usize;

        // Root planning: metadata + quick keyword probe, no AST text in root state.
        for meta in all_meta {
            if selected_files.len() >= budgets.max_chunks || consumed >= budgets.max_total_bytes {
                break;
            }
            let Ok(content) = fs::read_to_string(&meta.path) else {
                continue;
            };
            let lower = content.to_lowercase();
            let keyword_hit = keywords.iter().any(|k| lower.contains(&k.to_lowercase()));
            if !keyword_hit {
                continue;
            }

            let take = content.len().min(budgets.max_bytes_per_chunk);
            if consumed + take > budgets.max_total_bytes {
                break;
            }
            consumed += take;
            selected_files.push(meta.path);
        }

        let mut all_hits = Vec::new();
        if budgets.max_depth == 0 {
            return (all_hits, consumed);
        }

        for path in selected_files {
            if all_hits.len() >= budgets.max_chunks {
                break;
            }
            let mut hits = extract_hits_from_file(&path, keywords);
            all_hits.append(&mut hits);
        }

        all_hits.sort_by(|a, b| {
            a.file_path
                .cmp(&b.file_path)
                .then(a.line_start.cmp(&b.line_start))
                .then(a.name.cmp(&b.name))
        });
        (all_hits, consumed)
    }

    fn plan_files(root: &Path, budgets: Budgets, keywords: &[&str]) -> (Vec<PathBuf>, usize) {
        let all_meta = collect_metadata(root);
        let mut selected_files = Vec::new();
        let mut consumed = 0usize;

        for meta in all_meta {
            if selected_files.len() >= budgets.max_chunks || consumed >= budgets.max_total_bytes {
                break;
            }

            let Ok(content) = fs::read_to_string(&meta.path) else {
                continue;
            };
            let lower = content.to_lowercase();
            let keyword_hit = keywords.iter().any(|k| lower.contains(&k.to_lowercase()));
            if !keyword_hit {
                continue;
            }

            let take = content.len().min(budgets.max_bytes_per_chunk);
            if consumed + take > budgets.max_total_bytes {
                break;
            }
            consumed += take;
            selected_files.push(meta.path);
        }

        (selected_files, consumed)
    }

    fn mock_sub_call(hit: &SymbolHit) -> String {
        let first_doc_line = hit.doc.lines().next().unwrap_or("").trim();
        format!(
            "{}:{}:{}:{}:{}",
            hit.file_path.to_string_lossy(),
            hit.kind,
            hit.name,
            hit.line_start,
            first_doc_line
        )
    }

    fn assemble_output(hits: &[SymbolHit]) -> String {
        let mut rows: Vec<String> = hits.iter().map(mock_sub_call).collect();
        rows.sort();
        rows.join("\n")
    }

    fn relative_to_arrow(path: &Path) -> PathBuf {
        path.strip_prefix(ARROW_ROOT)
            .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
    }

    fn crate_name(path: &Path) -> String {
        relative_to_arrow(path)
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_default()
    }

    fn module_key(path: &Path) -> String {
        let rel = relative_to_arrow(path);
        let mut rel_no_ext = rel.clone();
        rel_no_ext.set_extension("");
        rel_no_ext.to_string_lossy().to_string()
    }

    fn categorize_reference(type_path: &Path, fn_path: &Path) -> &'static str {
        let t_in_ws = type_path.starts_with(ARROW_ROOT);
        let f_in_ws = fn_path.starts_with(ARROW_ROOT);
        if !(t_in_ws && f_in_ws) {
            return "external";
        }

        let t_mod = module_key(type_path);
        let f_mod = module_key(fn_path);
        if t_mod == f_mod {
            return "same_module";
        }

        let t_crate = crate_name(type_path);
        let f_crate = crate_name(fn_path);
        if t_crate == f_crate {
            return "other_module_same_crate";
        }

        "other_crate_workspace"
    }

    fn datafusion_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        let Ok(index_dirs) = fs::read_dir(CARGO_REGISTRY_SRC) else {
            return roots;
        };

        for index in index_dirs.filter_map(Result::ok) {
            let index_path = index.path();
            if !index_path.is_dir() {
                continue;
            }
            let Ok(crates) = fs::read_dir(&index_path) else {
                continue;
            };
            for crate_dir in crates.filter_map(Result::ok) {
                let p = crate_dir.path();
                if !p.is_dir() {
                    continue;
                }
                let Some(name) = p.file_name().map(|s| s.to_string_lossy().to_string()) else {
                    continue;
                };
                if name.starts_with("datafusion-") {
                    roots.push(p);
                }
            }
        }

        roots.sort();
        roots
    }

    fn scan_external_arrow_refs_in_datafusion(limit: usize) -> Vec<(PathBuf, usize, String)> {
        let mut refs = Vec::new();
        for root in datafusion_roots() {
            for file in rust_files(&root) {
                let Ok(content) = fs::read_to_string(&file) else {
                    continue;
                };
                for (idx, line) in content.lines().enumerate() {
                    let l = line.trim();
                    let hit = l.contains("use arrow")
                        || l.contains("arrow::")
                        || l.contains("arrow_")
                        || l.contains("DataType")
                        || l.contains("RecordBatch");
                    if hit {
                        refs.push((file.clone(), idx + 1, l.to_string()));
                        if refs.len() >= limit {
                            return refs;
                        }
                    }
                }
            }
        }
        refs
    }

    fn print_hit_samples(label: &str, hits: &[SymbolHit], n: usize) {
        println!("[{label}] total_hits={}", hits.len());
        for (idx, h) in hits.iter().take(n).enumerate() {
            let doc_line = h.doc.lines().next().unwrap_or("").trim();
            println!(
                "  sample#{idx}: {} {} {} L{}-L{} | sig={} | {}",
                h.file_path.to_string_lossy(),
                h.kind,
                h.name,
                h.line_start,
                h.line_end,
                h.signature,
                doc_line
            );
        }
    }

    #[test]
    fn spike_arrow_repo_scan() {
        let root = require_arrow_root();
        let metas = collect_metadata(&root);
        let file_count = metas.len();
        let total_lines: usize = metas.iter().map(|m| m.lines).sum();
        let total_bytes: usize = metas.iter().map(|m| m.bytes).sum();

        assert!(
            file_count > 200,
            "Expected large repo, got {file_count} Rust files"
        );
        assert!(
            total_lines > 100_000,
            "Expected large line count, got {total_lines}"
        );
        assert!(
            total_bytes > 3_000_000,
            "Expected multi-MB codebase, got {total_bytes}"
        );

        println!(
            "[repo_scan] rs_files={file_count} total_lines={total_lines} total_bytes={total_bytes}"
        );
    }

    #[test]
    fn spike_impl_query_trait_uuid() {
        let file = PathBuf::from(ARROW_ROOT).join("arrow-schema/src/extension/canonical/uuid.rs");
        let source = fs::read_to_string(&file).expect("uuid.rs should read");
        let count = run_impl_query(&source, extended_impl_query());
        assert!(
            count >= 1,
            "Extended query should capture impl ExtensionType for Uuid"
        );
    }

    #[test]
    fn spike_impl_query_generic_fields() {
        let file = PathBuf::from(ARROW_ROOT).join("arrow-schema/src/fields.rs");
        let source = fs::read_to_string(&file).expect("fields.rs should read");
        let count = run_impl_query(&source, extended_impl_query());
        assert!(
            count >= 1,
            "Extended query should capture generic impl<const N: usize> From<[FieldRef; N]> for Fields"
        );
    }

    #[test]
    fn spike_impl_query_delta() {
        let root = require_arrow_root();
        let files = rust_files(&root);

        let mut baseline = 0usize;
        let mut extended = 0usize;
        for path in files {
            let Ok(source) = fs::read_to_string(&path) else {
                continue;
            };
            baseline += run_impl_query(&source, baseline_impl_query());
            extended += run_impl_query(&source, extended_impl_query());
        }

        assert!(
            extended >= baseline,
            "Extended impl query should cover at least baseline (baseline={baseline}, extended={extended})"
        );

        println!(
            "[impl_delta] baseline_impl_matches={baseline} extended_impl_matches={extended} delta={}",
            extended.saturating_sub(baseline)
        );
    }

    #[test]
    fn spike_ast_tree_preview() {
        let file = PathBuf::from(ARROW_ROOT).join("arrow-schema/src/fields.rs");
        let preview = ast_tree_sexp_preview(&file, 1200);
        let source = fs::read_to_string(&file).expect("fields.rs should read");
        let impl_count = run_impl_query(&source, "(impl_item) @impl");
        println!(
            "[ast_tree_preview] file={}\n{}",
            file.to_string_lossy(),
            preview
        );

        assert!(preview.contains("source_file"));
        assert!(impl_count > 0, "Expected at least one impl_item in AST");
    }

    #[test]
    fn spike_doc_span_extraction() {
        let file = PathBuf::from(ARROW_ROOT).join("parquet/src/arrow/arrow_reader/selection.rs");
        let hits = extract_hits_from_file(&file, &["invariant"]);
        assert!(
            !hits.is_empty(),
            "Expected at least one symbol with invariant doc comments in selection.rs"
        );
        assert!(
            hits.iter()
                .any(|h| h.name == "RowSelection" && h.doc.to_lowercase().contains("invariant")),
            "Expected RowSelection invariant docs to be captured"
        );

        print_hit_samples("doc_span_extraction", &hits, 3);
    }

    #[test]
    fn spike_recursive_filter_ast_and_docs() {
        let file = PathBuf::from(ARROW_ROOT).join("parquet/src/arrow/arrow_reader/selection.rs");
        let hits = extract_hits_from_file(&file, &["invariant", "panic"]);
        assert!(!hits.is_empty(), "AST+doc filter should produce hits");
        assert!(
            hits.iter().all(|h| {
                let d = h.doc.to_lowercase();
                d.contains("invariant") || d.contains("panic")
            }),
            "All hits must satisfy doc keyword filter"
        );
    }

    #[test]
    fn spike_recursive_sub_call_dispatch() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 40,
            max_bytes_per_chunk: 4_000,
            max_total_bytes: 120_000,
        };
        let (hits, _) = build_linear_summary(&root, budgets, &["invariant", "panic", "safety"]);
        let sample: Vec<SymbolHit> = hits.into_iter().take(25).collect();
        assert!(
            !sample.is_empty(),
            "Need at least one hit for dispatch test"
        );

        let outputs: Vec<String> = sample.iter().map(mock_sub_call).collect();
        assert_eq!(
            outputs.len(),
            sample.len(),
            "Each selected slice should dispatch exactly one sub-call"
        );
        assert!(outputs.iter().all(|o| o.contains(':')));

        let idx = build_signature_index(&sample);
        let first = &sample[0];
        let sig = signature_for_hit(first, &idx).unwrap_or_default();
        assert!(
            !sig.is_empty(),
            "signature lookup should return non-empty value"
        );
    }

    #[test]
    fn spike_recursive_output_assembly() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 50,
            max_bytes_per_chunk: 4_000,
            max_total_bytes: 160_000,
        };
        let (hits, _) = build_linear_summary(&root, budgets, &["invariant", "panic", "safety"]);
        let sample: Vec<SymbolHit> = hits.into_iter().take(30).collect();
        assert!(!sample.is_empty(), "Need hits to assemble output");

        let assembled = assemble_output(&sample);
        let row_count = assembled.lines().count();
        assert_eq!(
            row_count,
            sample.len(),
            "Output must contain one row per hit"
        );
        assert!(
            assembled.contains("arrow-rs"),
            "Output should include full file path context"
        );

        println!(
            "[output_assembly] rows={} sample:\n{}",
            row_count,
            assembled.lines().take(3).collect::<Vec<_>>().join("\n")
        );
    }

    #[test]
    fn spike_recursive_metadata_only_root_and_budget() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 40,
            max_bytes_per_chunk: 6_000,
            max_total_bytes: 120_000,
        };
        let keywords = ["safety", "panic", "invariant"];

        let (hits, consumed) = build_linear_summary(&root, budgets, &keywords);
        assert!(consumed <= budgets.max_total_bytes);
        assert!(
            hits.len() <= 2_000,
            "Unexpectedly large hit volume in budgeted run"
        );
    }

    #[test]
    fn spike_recursive_linear_task() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 200,
            max_bytes_per_chunk: 6_000,
            max_total_bytes: 750_000,
        };
        let keywords = ["safety", "panic", "invariant"];

        let (hits, _) = build_linear_summary(&root, budgets, &keywords);
        assert!(!hits.is_empty(), "Linear task should return non-empty hits");

        // Ensure we span multiple files to prove broad-context processing.
        let unique_files: BTreeSet<_> = hits.iter().map(|h| h.file_path.clone()).collect();
        assert!(
            unique_files.len() >= 5,
            "Expected broad coverage across files, got {}",
            unique_files.len()
        );

        println!(
            "[linear_task] unique_files={} total_hits={} keywords=safety|panic|invariant",
            unique_files.len(),
            hits.len()
        );
        print_hit_samples("linear_task", &hits, 5);
    }

    #[test]
    fn spike_recursive_pairwise_task() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 250,
            max_bytes_per_chunk: 6_000,
            max_total_bytes: 750_000,
        };
        let (hits, _) = build_linear_summary(&root, budgets, &["invariant", "panic", "safety"]);

        let mut type_hits = Vec::new();
        let mut fn_hits = Vec::new();
        for h in hits {
            match h.kind.as_str() {
                "struct" | "enum" | "trait" => {
                    let d = h.doc.to_lowercase();
                    if d.contains("invariant") || d.contains("safety") || d.contains("panic") {
                        type_hits.push(h);
                    }
                }
                "function" => {
                    let d = h.doc.to_lowercase();
                    if d.contains("panic") || d.contains("safe") || d.contains("invariant") {
                        fn_hits.push(h);
                    }
                }
                _ => {}
            }
        }

        assert!(
            !type_hits.is_empty(),
            "Pairwise task requires at least one type hit with relevant docs"
        );
        assert!(
            !fn_hits.is_empty(),
            "Pairwise task requires at least one function hit with safety/panic/invariant docs"
        );

        // Pairing heuristic for the spike: require at least one shared thematic
        // keyword between docs, then categorize pair locality.
        let mut pairs = Vec::new();
        for t in type_hits.iter().take(50) {
            let t_doc = t.doc.to_lowercase();
            for f in fn_hits.iter().take(200) {
                let f_doc = f.doc.to_lowercase();
                let shared_theme = ["invariant", "safety", "panic"]
                    .iter()
                    .any(|k| t_doc.contains(k) && f_doc.contains(k));

                if shared_theme {
                    let category = categorize_reference(&t.file_path, &f.file_path);
                    pairs.push((
                        t.file_path.clone(),
                        t.name.clone(),
                        f.file_path.clone(),
                        f.name.clone(),
                        category.to_string(),
                    ));
                }
            }
        }

        assert!(
            !pairs.is_empty(),
            "Pairwise task should produce at least one pair"
        );

        let mut category_counts: BTreeMap<String, usize> = BTreeMap::new();
        for (_, _, _, _, cat) in &pairs {
            *category_counts.entry(cat.clone()).or_default() += 1;
        }

        let sig_index = build_signature_index(&type_hits);
        let fn_sig_index = build_signature_index(&fn_hits);

        let external_refs = scan_external_arrow_refs_in_datafusion(40);
        if !external_refs.is_empty() {
            category_counts.insert("external".to_string(), external_refs.len());
        }

        println!(
            "[pairwise_task] type_hits={} fn_hits={} pairs={} categories={:?}",
            type_hits.len(),
            fn_hits.len(),
            pairs.len(),
            category_counts
        );
        for (idx, (t_file, t_name, f_file, f_name, cat)) in pairs.iter().take(8).enumerate() {
            println!(
                "  pair#{idx} [{cat}]: type={}::{}, fn={}::{}",
                t_file.to_string_lossy(),
                t_name,
                f_file.to_string_lossy(),
                f_name
            );
        }

        println!("[external_refs:datafusion] hits={}", external_refs.len());
        for (idx, (path, line_no, line)) in external_refs.iter().take(6).enumerate() {
            println!(
                "  external#{idx} [external]: {}:{} | {}",
                path.to_string_lossy(),
                line_no,
                line
            );
        }

        let mut top_external_files: BTreeMap<String, usize> = BTreeMap::new();
        for (path, _, _) in &external_refs {
            *top_external_files
                .entry(path.to_string_lossy().to_string())
                .or_default() += 1;
        }

        let mut top_external_files_vec: Vec<(String, usize)> =
            top_external_files.into_iter().collect();
        top_external_files_vec.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        let pair_samples: Vec<serde_json::Value> = pairs
            .iter()
            .take(5)
            .map(|(t_file, t_name, f_file, f_name, cat)| {
                let t_hit = type_hits
                    .iter()
                    .find(|h| &h.file_path == t_file && &h.name == t_name)
                    .cloned();
                let f_hit = fn_hits
                    .iter()
                    .find(|h| &h.file_path == f_file && &h.name == f_name)
                    .cloned();

                let t_sig = t_hit
                    .as_ref()
                    .and_then(|h| signature_for_hit(h, &sig_index))
                    .unwrap_or_default();
                let f_sig = f_hit
                    .as_ref()
                    .and_then(|h| signature_for_hit(h, &fn_sig_index))
                    .unwrap_or_default();

                json!({
                    "category": cat,
                    "type": {
                        "file": t_file.to_string_lossy().to_string(),
                        "name": t_name,
                        "signature": t_sig,
                    },
                    "function": {
                        "file": f_file.to_string_lossy().to_string(),
                        "name": f_name,
                        "signature": f_sig,
                    }
                })
            })
            .collect();

        let external_samples: Vec<serde_json::Value> = external_refs
            .iter()
            .take(6)
            .map(|(path, line_no, line)| {
                json!({
                    "file": path.to_string_lossy().to_string(),
                    "line": line_no,
                    "text": line,
                })
            })
            .collect();

        let top_external_files_json: Vec<serde_json::Value> = top_external_files_vec
            .into_iter()
            .take(5)
            .map(|(file, hits)| json!({ "file": file, "hits": hits }))
            .collect();

        let summary = json!({
            "spike": "0.21",
            "task": "pairwise_reference_categorization",
            "counts": {
                "type_hits": type_hits.len(),
                "function_hits": fn_hits.len(),
                "pairs": pairs.len(),
                "external_refs": external_refs.len(),
            },
            "categories": category_counts,
            "top_external_files": top_external_files_json,
            "pair_samples": pair_samples,
            "external_samples": external_samples,
        });

        println!("[summary_json] {}", summary);
        let summary_pretty =
            serde_json::to_string_pretty(&summary).expect("summary json should pretty serialize");
        println!("[summary_json_pretty]\n{}", summary_pretty);

        assert!(
            category_counts.contains_key("same_module")
                || category_counts.contains_key("other_module_same_crate")
                || category_counts.contains_key("other_crate_workspace")
                || category_counts.contains_key("external"),
            "Expected at least one reference category"
        );

        if !datafusion_roots().is_empty() {
            assert!(
                !external_refs.is_empty(),
                "Expected to find at least one Arrow reference in cached datafusion crates"
            );
        }
    }

    #[test]
    fn spike_recursive_budget_max_chunks() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 3,
            max_bytes_per_chunk: 10_000,
            max_total_bytes: 100_000,
        };
        let (selected, _) = plan_files(&root, budgets, &["use"]);
        assert_eq!(selected.len(), 3, "Planner must stop at max_chunks");
    }

    #[test]
    fn spike_recursive_budget_max_bytes() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 15,
            max_bytes_per_chunk: 128,
            max_total_bytes: 4_096,
        };
        let (selected, consumed) = plan_files(&root, budgets, &["use"]);
        assert!(
            !selected.is_empty(),
            "Expected planner to select some files"
        );
        assert!(
            consumed <= selected.len() * budgets.max_bytes_per_chunk,
            "Consumed bytes should respect per-chunk cap"
        );
    }

    #[test]
    fn spike_recursive_budget_total() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 20,
            max_bytes_per_chunk: 800,
            max_total_bytes: 1_000,
        };
        let (_selected, consumed) = plan_files(&root, budgets, &["use"]);
        assert!(
            consumed <= budgets.max_total_bytes,
            "Planner must stop when total byte budget is reached"
        );
    }

    #[test]
    fn spike_recursive_stability() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 120,
            max_bytes_per_chunk: 4_000,
            max_total_bytes: 300_000,
        };
        let keywords = ["safety", "panic", "invariant"];

        let (a_hits, _) = build_linear_summary(&root, budgets, &keywords);
        let (b_hits, _) = build_linear_summary(&root, budgets, &keywords);

        let a: Vec<_> = a_hits
            .iter()
            .map(|h| {
                (
                    h.file_path.to_string_lossy().to_string(),
                    h.kind.clone(),
                    h.name.clone(),
                    h.line_start,
                    h.line_end,
                )
            })
            .collect();
        let b: Vec<_> = b_hits
            .iter()
            .map(|h| {
                (
                    h.file_path.to_string_lossy().to_string(),
                    h.kind.clone(),
                    h.name.clone(),
                    h.line_start,
                    h.line_end,
                )
            })
            .collect();

        assert_eq!(a, b, "Recursive query results should be deterministic");
    }

    #[test]
    fn spike_reference_graph_persistence() {
        let root = require_arrow_root();
        let budgets = Budgets {
            max_depth: 2,
            max_chunks: 180,
            max_bytes_per_chunk: 6_000,
            max_total_bytes: 650_000,
        };
        let (hits, _) = build_linear_summary(&root, budgets, &["invariant", "panic", "safety"]);
        assert!(!hits.is_empty(), "Need hits to build graph");

        let type_hits: Vec<SymbolHit> = hits
            .iter()
            .filter(|h| matches!(h.kind.as_str(), "struct" | "enum" | "trait"))
            .cloned()
            .collect();
        let fn_hits: Vec<SymbolHit> = hits
            .iter()
            .filter(|h| h.kind == "function")
            .cloned()
            .collect();

        assert!(!type_hits.is_empty(), "Need type hits");
        assert!(!fn_hits.is_empty(), "Need function hits");

        let mut edges = build_internal_edges(&type_hits, &fn_hits, 25, 120);
        assert!(!edges.is_empty(), "Need internal edges");

        let external_refs = scan_external_arrow_refs_in_datafusion(20);
        for (path, line_no, line) in &external_refs {
            if let Some(target) = fn_hits.first() {
                let source_ref = format!("external::{}:{}", path.to_string_lossy(), line_no);
                let target_ref = hit_ref_id(target);
                let evidence = format!("line={} text={}", line_no, line);
                edges.push((source_ref, target_ref, "external".to_string(), evidence));
            }
        }

        let conn = setup_ref_graph_db();

        for h in &hits {
            let ref_id = hit_ref_id(h);
            conn.execute(
                "INSERT OR REPLACE INTO symbol_refs
                 (ref_id, file_path, kind, name, line_start, line_end, signature, doc)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    ref_id,
                    h.file_path.to_string_lossy().to_string(),
                    h.kind,
                    h.name,
                    h.line_start as i64,
                    h.line_end as i64,
                    h.signature,
                    h.doc
                ],
            )
            .expect("insert symbol ref should succeed");
        }

        for (idx, (source_ref, target_ref, category, evidence)) in edges.iter().enumerate() {
            let edge_id = format!("edge-{}", idx);
            conn.execute(
                "INSERT INTO ref_edges (edge_id, source_ref_id, target_ref_id, category, evidence)
                 VALUES (?, ?, ?, ?, ?)",
                params![edge_id, source_ref, target_ref, category, evidence],
            )
            .expect("insert edge should succeed");
        }

        let ref_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM symbol_refs", [], |row| row.get(0))
            .expect("count refs should succeed");
        let edge_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ref_edges", [], |row| row.get(0))
            .expect("count edges should succeed");
        assert!(ref_count > 0);
        assert!(edge_count > 0);

        // Signature lookup by ref id via DB.
        let sample_hit = &hits[0];
        let sample_ref = hit_ref_id(sample_hit);
        let sig = signature_lookup_db(&conn, &sample_ref).unwrap_or_default();
        assert!(!sig.is_empty(), "DB signature lookup should return value");

        let mut stmt = conn
            .prepare("SELECT category, COUNT(*) FROM ref_edges GROUP BY category ORDER BY category")
            .expect("prepare category stats should succeed");
        let category_stats: Vec<(String, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query category stats should succeed")
            .filter_map(Result::ok)
            .collect();

        println!(
            "[ref_graph] refs={} edges={} category_stats={:?} sample_ref={} sample_signature={}",
            ref_count, edge_count, category_stats, sample_ref, sig
        );
    }
}
