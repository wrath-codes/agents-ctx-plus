//! # ast-grep Spike (Task 0.8)
//!
//! **Validates**: `ast-grep-core` 0.40.5 + `ast-grep-language` 0.40.5 work together
//!   for parsing, pattern matching, traversal, and composable matchers across all 7
//!   rich extractor languages (Rust, Python, TypeScript, JavaScript, Go, Elixir, TSX).
//!
//! **Blocks**: Phase 3 (Parsing & Indexing Pipeline)
//!
//! **Critical path**: Phase 0 → Phase 3 → Phase 4 → Phase 5 (MVP)
//!
//! ## API Notes (ast-grep 0.40.x — Rust API is NOT stable)
//!
//! - Only 23% of `ast-grep-core` is documented. We're working from source + docs.rs.
//! - `node.text()` and `node.kind()` return `Cow<str>`, not `String` or `&str`.
//! - `Position.line` is zero-based character offsets (differs from tree-sitter byte offsets).
//! - Metavar names strip `$` prefix: `get_match("FNAME")` not `get_match("$FNAME")`.
//! - `$$$` multi-metavars need `get_multiple_matches()` — `get_match()` returns `None`.
//! - Rust/Go/C use expando char for `$` since `$` isn't valid in those identifiers.
//! - `KindMatcher::new()` may be fallible — spike confirms.
//! - `LanguageExt` is NOT dyn-compatible (not object safe).
//! - `SupportLang` is `Copy` — free to pass by value.
//! - `NodeMatch` derefs to `Node` — can call Node methods directly on match results.
//! - klaw-effect-tracker uses text-based doc comment extraction; our AST sibling
//!   approach via `node.prev()` needs validation.
//! - klaw detects async via child walking, not text matching.
//!
//! ## Fallback Paths (documented but NOT tested in this spike)
//!
//! 1. **Raw tree-sitter crate** (transitive dep via ast-grep-language): `Parser::new()`,
//!    `Query::new()`, `QueryCursor` with S-expression patterns.
//! 2. **WASM grammar loading**: `tree-sitter` crate `wasm` feature + `wasmtime`. 36
//!    pre-built grammars available via `tree-sitter-wasms` npm package.
//! 3. **ast-grep custom Language trait**: implement `Language` + load `.so`/`.dylib`
//!    dynamic library for languages beyond the 26 built-in.

#[cfg(test)]
mod tests {
    use ast_grep_core::matcher::KindMatcher;
    use ast_grep_core::ops::{Any, Not};
    use ast_grep_language::{LanguageExt, SupportLang};

    // =========================================================================
    // Section 1: Core Parsing
    // =========================================================================

    #[test]
    fn spike_parse_rust_source() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);
        let node = root.root();

        // Root should be "source_file" kind
        assert_eq!(node.kind().as_ref(), "source_file");

        // Should have children (our fixture has functions, structs, etc.)
        let children: Vec<_> = node.children().collect();
        assert!(
            children.len() > 5,
            "Expected many top-level items, got {}",
            children.len()
        );
    }

    #[test]
    fn spike_parse_all_rich_languages() {
        let cases: &[(&str, SupportLang, &str)] = &[
            (
                include_str!("../tests/fixtures/sample.rs"),
                SupportLang::Rust,
                "source_file",
            ),
            (
                include_str!("../tests/fixtures/sample.py"),
                SupportLang::Python,
                "module",
            ),
            (
                include_str!("../tests/fixtures/sample.ts"),
                SupportLang::TypeScript,
                "program",
            ),
            (
                include_str!("../tests/fixtures/sample.js"),
                SupportLang::JavaScript,
                "program",
            ),
            (
                include_str!("../tests/fixtures/sample.go"),
                SupportLang::Go,
                "source_file",
            ),
            (
                include_str!("../tests/fixtures/sample.ex"),
                SupportLang::Elixir,
                "source",
            ),
            (
                include_str!("../tests/fixtures/sample.tsx"),
                SupportLang::Tsx,
                "program",
            ),
        ];

        for (source, lang, expected_root_kind) in cases {
            let root = lang.ast_grep(source);
            let node = root.root();
            assert_eq!(
                node.kind().as_ref(),
                *expected_root_kind,
                "Root kind mismatch for {:?}",
                lang
            );
            let children: Vec<_> = node.children().collect();
            assert!(!children.is_empty(), "{:?} parsed with no children", lang);
        }
    }

    #[test]
    fn spike_language_detection_from_extension() {
        let cases = &[
            ("rs", SupportLang::Rust),
            ("py", SupportLang::Python),
            ("ts", SupportLang::TypeScript),
            ("js", SupportLang::JavaScript),
            ("go", SupportLang::Go),
            ("ex", SupportLang::Elixir),
            ("tsx", SupportLang::Tsx),
            // Also test common aliases
            ("rust", SupportLang::Rust),
            ("python", SupportLang::Python),
            ("typescript", SupportLang::TypeScript),
            ("javascript", SupportLang::JavaScript),
            ("golang", SupportLang::Go),
            ("elixir", SupportLang::Elixir),
        ];

        for (ext, expected) in cases {
            let lang: SupportLang = ext
                .parse()
                .unwrap_or_else(|_| panic!("Failed to parse {:?} as SupportLang", ext));
            assert_eq!(lang, *expected, "Extension {:?} mismatch", ext);
        }
    }

    // =========================================================================
    // Section 2: Pattern Matching & MetaVariables
    // =========================================================================

    #[test]
    fn spike_pattern_match_rust_functions() {
        // FINDING: Pattern-based matching for Rust functions is limited because:
        // 1. Pattern must be syntactically valid — can't have partial syntax
        // 2. Patterns must match the exact structure including return types
        // 3. `fn $FNAME($$$PARAMS) { $$$ }` does NOT match functions with return types
        //
        // This confirms that Phase 3 should use KindMatcher as primary strategy,
        // with patterns only for specific structural queries.

        // Test with simple functions (no return type) — pattern works
        let simple_source = "fn hello() { } fn world() { }";
        let root = SupportLang::Rust.ast_grep(simple_source);
        let simple_matches: Vec<_> = root.root().find_all("fn $FNAME() { $$$ }").collect();
        assert_eq!(
            simple_matches.len(),
            2,
            "Simple pattern should match both functions"
        );

        // Test with fixture file — use KindMatcher instead (the klaw approach)
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);
        let fn_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let kind_matches: Vec<_> = root.root().find_all(fn_matcher).collect();
        assert!(
            kind_matches.len() >= 4,
            "KindMatcher should find all functions, got {}",
            kind_matches.len()
        );

        // Verify we can still get names from kind-matched nodes
        let names: Vec<String> = kind_matches
            .iter()
            .filter_map(|m| m.field("name").map(|n| n.text().to_string()))
            .collect();
        println!(
            "FINDING: KindMatcher found {} functions: {:?}. \
             Pattern-based matching is fragile for Rust (return types, generics break it). \
             Use KindMatcher as primary, patterns for specific structural queries only.",
            kind_matches.len(),
            names
        );
        assert!(names.contains(&"process".to_string()));
        assert!(names.contains(&"dangerous".to_string()));
    }

    #[test]
    fn spike_metavariable_single_capture() {
        let source = "fn hello() {} fn world() {}";
        let root = SupportLang::Rust.ast_grep(source);

        let matches: Vec<_> = root.root().find_all("fn $FNAME() { $$$ }").collect();
        assert_eq!(matches.len(), 2, "Should match both functions");

        let names: Vec<String> = matches
            .iter()
            .map(|m| {
                m.get_env()
                    .get_match("FNAME")
                    .expect("FNAME should be captured")
                    .text()
                    .to_string()
            })
            .collect();

        assert!(names.contains(&"hello".to_string()));
        assert!(names.contains(&"world".to_string()));
        println!("FINDING: Single metavar capture works. Names: {:?}", names);
    }

    #[test]
    fn spike_metavariable_multi_capture() {
        // Use function WITHOUT return type so pattern matches
        // (return type in source breaks the pattern — key finding from test 4)
        let source = "fn add(a: i32, b: i32) { }";
        let root = SupportLang::Rust.ast_grep(source);

        let matches: Vec<_> = root
            .root()
            .find_all("fn $FNAME($$$PARAMS) { $$$ }")
            .collect();
        assert!(!matches.is_empty(), "Should match the function");

        let first = &matches[0];
        let env = first.get_env();

        // Single metavar for function name
        let fname = env.get_match("FNAME");
        assert!(fname.is_some(), "FNAME should be captured as single");
        assert_eq!(fname.unwrap().text().as_ref(), "add");

        // Multi-metavar for params — should use get_multiple_matches
        let params = env.get_multiple_matches("PARAMS");
        println!(
            "FINDING: Multi-metavar PARAMS captured {} nodes",
            params.len()
        );
        // get_match on a multi-metavar may return None or the first — document behavior
        let single_try = env.get_match("PARAMS");
        println!(
            "FINDING: get_match(\"PARAMS\") on $$$ var returns: {:?}",
            single_try.map(|n| n.text().to_string())
        );
    }

    #[test]
    fn spike_pattern_match_python_classes() {
        let source = include_str!("../tests/fixtures/sample.py");
        let root = SupportLang::Python.ast_grep(source);

        // Try matching class definitions
        let matches: Vec<_> = root.root().find_all("class $NAME: $$$BODY").collect();
        // This may or may not work — Python classes with base classes have different
        // syntax. Let's also try kind-based.
        println!(
            "FINDING: Pattern 'class $NAME: $$$BODY' matched {} nodes",
            matches.len()
        );

        // Fallback: use KindMatcher for class_definition
        let class_matcher = KindMatcher::new("class_definition", SupportLang::Python);
        let kind_matches: Vec<_> = root.root().find_all(class_matcher).collect();
        assert!(
            kind_matches.len() >= 2,
            "Should find at least BaseProcessor and Config classes, got {}",
            kind_matches.len()
        );
        for m in &kind_matches {
            let name = m.field("name").map(|n| n.text().to_string());
            println!("FINDING: Python class found: {:?}", name);
        }
    }

    // =========================================================================
    // Section 3: Composable Matchers
    // =========================================================================

    #[test]
    fn spike_kind_matcher_with_any() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        // Create KindMatchers for multiple node types
        let fn_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let struct_matcher = KindMatcher::new("struct_item", SupportLang::Rust);
        let enum_matcher = KindMatcher::new("enum_item", SupportLang::Rust);

        // Combine with Any
        let any_matcher = Any::new(vec![fn_matcher, struct_matcher, enum_matcher]);

        let matches: Vec<_> = root.root().find_all(any_matcher).collect();
        // Our fixture has: process (fn), dangerous (fn), Config (struct), Status (enum),
        // handle (fn in impl), new (fn in impl) — functions in impl may or may not be
        // found at top level
        assert!(
            matches.len() >= 4,
            "Should find functions + structs + enums, got {}",
            matches.len()
        );

        let kinds: Vec<String> = matches.iter().map(|m| m.kind().to_string()).collect();
        println!(
            "FINDING: Any combinator matched {} nodes. Kinds: {:?}",
            matches.len(),
            kinds
        );
        assert!(kinds.contains(&"function_item".to_string()));
        assert!(kinds.contains(&"struct_item".to_string()));
        assert!(kinds.contains(&"enum_item".to_string()));
    }

    #[test]
    fn spike_all_combinator() {
        // All::new() requires all matchers to be the same type.
        // This means All<KindMatcher> can only combine KindMatchers.
        // For mixed types, ast-grep uses Op<L> which wraps Box<dyn Matcher>.
        //
        // Test: All with two KindMatchers for an impossible combination
        // (nothing is both function_item AND struct_item)
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        // All functions first
        let kind_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let all_fns: Vec<_> = root.root().find_all(kind_matcher).collect();
        let all_fn_count = all_fns.len();

        // All with impossible combination: function_item AND struct_item
        use ast_grep_core::ops::All;
        let all_matcher = All::new(vec![
            KindMatcher::new("function_item", SupportLang::Rust),
            KindMatcher::new("struct_item", SupportLang::Rust),
        ]);
        let impossible: Vec<_> = root.root().find_all(all_matcher).collect();
        assert_eq!(
            impossible.len(),
            0,
            "Nothing should be both function_item and struct_item"
        );

        // All with same kind twice should match same as single kind
        let all_same = All::new(vec![
            KindMatcher::new("function_item", SupportLang::Rust),
            KindMatcher::new("function_item", SupportLang::Rust),
        ]);
        let same_matches: Vec<_> = root.root().find_all(all_same).collect();
        assert_eq!(
            same_matches.len(),
            all_fn_count,
            "All(fn, fn) should match same count as single fn matcher"
        );

        println!(
            "FINDING: All combinator works. Impossible=0, Same-kind={}, Total fns={}. \
             NOTE: All::new() requires homogeneous matcher types (Vec<M> where M: Matcher). \
             For mixed types (e.g., KindMatcher + pattern), use ast_grep_core::ops::Op.",
            same_matches.len(),
            all_fn_count
        );
    }

    #[test]
    fn spike_not_combinator() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        // Count all function_items
        let fn_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let all_fns: Vec<_> = root.root().find_all(fn_matcher).collect();

        // Now find function_items that are NOT inside impl blocks
        // Use Not to exclude impl_items
        let _fn_matcher2 = KindMatcher::new("function_item", SupportLang::Rust);
        let impl_matcher = KindMatcher::new("impl_item", SupportLang::Rust);
        let not_impl = Not::new(impl_matcher);

        // Not matcher alone: find all nodes that are NOT impl_item
        let not_impl_matches: Vec<_> = root.root().find_all(not_impl).collect();

        println!(
            "FINDING: Not combinator - total functions: {}, \
             nodes that are NOT impl_item: {}",
            all_fns.len(),
            not_impl_matches.len()
        );

        // Not should exclude impl_items — the count should be greater than 0
        // and less than total child count (since some ARE impl_items)
        assert!(
            !not_impl_matches.is_empty(),
            "Not combinator should match something"
        );

        // Verify that no impl_items are in the Not results
        let has_impl = not_impl_matches
            .iter()
            .any(|m| m.kind().as_ref() == "impl_item");
        // Not::new inverts — when used with find_all it finds nodes where the
        // inner matcher does NOT match. But Not is not "positive" by itself,
        // so it may behave differently. Let's just document what happens.
        println!(
            "FINDING: Not results contain impl_item: {} (total: {})",
            has_impl,
            not_impl_matches.len()
        );

        // NOTE: All::new() requires homogeneous types, so we can't mix
        // KindMatcher with Not<KindMatcher> directly. For mixed types,
        // ast-grep uses Op<L> which boxes matchers. This is a key finding
        // for Phase 3: use Op for complex composed matchers.
        println!(
            "FINDING: Cannot mix KindMatcher and Not<KindMatcher> in All::new() — \
             types must be homogeneous. Use ast_grep_core::ops::Op for mixed types."
        );
    }

    // =========================================================================
    // Section 4: Node Traversal — klaw Extraction Patterns
    // =========================================================================

    #[test]
    fn spike_node_field_access() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        // Find the first function_item
        let fn_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let first_fn = root
            .root()
            .find(fn_matcher)
            .expect("Should find at least one function");

        // Field access: name
        let name_node = first_fn.field("name");
        assert!(name_node.is_some(), "Function should have a 'name' field");
        let name_text = name_node.unwrap().text().to_string();
        println!("FINDING: field(\"name\") = {:?}", name_text);

        // Field access: parameters
        let params_node = first_fn.field("parameters");
        assert!(
            params_node.is_some(),
            "Function should have a 'parameters' field"
        );
        println!(
            "FINDING: field(\"parameters\") = {:?}",
            params_node.map(|n| n.text().to_string())
        );

        // Field access: return_type
        let ret_node = first_fn.field("return_type");
        println!(
            "FINDING: field(\"return_type\") = {:?}",
            ret_node.map(|n| n.text().to_string())
        );

        // Field access: body
        let body_node = first_fn.field("body");
        assert!(body_node.is_some(), "Function should have a 'body' field");

        // Now test impl block fields — trait vs type for inherent vs trait impl
        let impl_matcher = KindMatcher::new("impl_item", SupportLang::Rust);
        let impls: Vec<_> = root.root().find_all(impl_matcher).collect();
        assert!(
            impls.len() >= 2,
            "Should find at least 2 impl blocks, got {}",
            impls.len()
        );

        for imp in &impls {
            let trait_field = imp.field("trait");
            let type_field = imp.field("type");
            let body_field = imp.field("body");
            println!(
                "FINDING: impl block — trait={:?}, type={:?}, has_body={}",
                trait_field.map(|n| n.text().to_string()),
                type_field.map(|n| n.text().to_string()),
                body_field.is_some()
            );
        }
    }

    #[test]
    fn spike_sibling_walking_for_doc_comments() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        // Find the `process` function which has doc comments above it
        let fn_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let first_fn = root
            .root()
            .find(fn_matcher)
            .expect("Should find a function");

        // Walk backward through siblings to find doc comments
        // This is the klaw pattern: node.prev() repeatedly
        let mut comments = Vec::new();
        let mut current = first_fn.prev();
        while let Some(sibling) = current {
            let kind = sibling.kind();
            let text = sibling.text().to_string();
            println!(
                "FINDING: prev sibling — kind={:?}, text={:?}",
                kind.as_ref(),
                &text[..text.len().min(60)]
            );

            if kind.as_ref() == "line_comment" {
                if text.starts_with("///") || text.starts_with("//!") {
                    comments.push(
                        text.trim_start_matches("///")
                            .trim_start_matches("//!")
                            .trim()
                            .to_string(),
                    );
                } else {
                    break;
                }
            } else if kind.as_ref() == "attribute_item" {
                // Skip attributes, keep looking for docs (klaw pattern)
            } else {
                break;
            }
            current = sibling.prev();
        }
        comments.reverse();
        let doc = comments.join("\n");
        println!("FINDING: Extracted doc comment: {:?}", doc);
        // The `process` function should have doc comments
        assert!(
            !doc.is_empty(),
            "Should extract doc comments from above function"
        );
        assert!(
            doc.contains("documented async function"),
            "Doc should mention 'documented async function', got: {:?}",
            doc
        );
    }

    #[test]
    fn spike_children_for_modifiers_and_members() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        // Test 1: Find async/unsafe by walking children of function nodes
        let fn_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let fns: Vec<_> = root.root().find_all(fn_matcher).collect();

        for f in &fns {
            let name = f
                .field("name")
                .map(|n| n.text().to_string())
                .unwrap_or_default();
            let children: Vec<_> = f.children().collect();
            let child_kinds: Vec<String> = children.iter().map(|c| c.kind().to_string()).collect();

            // Check for async/unsafe in children or in function_modifiers
            let has_async = children.iter().any(|c| c.kind().as_ref() == "async");
            let has_unsafe = children.iter().any(|c| c.kind().as_ref() == "unsafe");
            let has_modifiers = children
                .iter()
                .any(|c| c.kind().as_ref() == "function_modifiers");

            // Also check text-based detection for comparison
            let text = f.text();
            let text_has_async = text.contains("async");
            let text_has_unsafe = text.contains("unsafe");

            println!(
                "FINDING: fn {} — child-async={}, child-unsafe={}, has_modifiers={}, \
                 text-async={}, text-unsafe={}, child_kinds={:?}",
                name,
                has_async,
                has_unsafe,
                has_modifiers,
                text_has_async,
                text_has_unsafe,
                child_kinds
            );
        }

        // Test 2: Enum variants via children
        let enum_matcher = KindMatcher::new("enum_item", SupportLang::Rust);
        let enums: Vec<_> = root.root().find_all(enum_matcher).collect();
        assert!(!enums.is_empty(), "Should find at least one enum");

        let body = enums[0].field("body");
        assert!(body.is_some(), "Enum should have a body");
        let variants: Vec<String> = body
            .unwrap()
            .children()
            .filter(|c| c.kind().as_ref() == "enum_variant")
            .filter_map(|v| v.field("name").map(|n| n.text().to_string()))
            .collect();
        println!("FINDING: Enum variants: {:?}", variants);
        assert!(variants.contains(&"Active".to_string()));

        // Test 3: Struct fields via children
        let struct_matcher = KindMatcher::new("struct_item", SupportLang::Rust);
        let structs: Vec<_> = root.root().find_all(struct_matcher).collect();
        assert!(!structs.is_empty(), "Should find at least one struct");

        let body = structs[0].field("body");
        assert!(body.is_some(), "Struct should have a body");
        let fields: Vec<String> = body
            .unwrap()
            .children()
            .filter(|c| c.kind().as_ref() == "field_declaration")
            .filter_map(|f| f.field("name").map(|n| n.text().to_string()))
            .collect();
        println!("FINDING: Struct fields: {:?}", fields);
        assert!(!fields.is_empty(), "Should find struct fields");
    }

    #[test]
    fn spike_node_parent_and_ancestors() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        // Find a function inside an impl block
        let impl_matcher = KindMatcher::new("impl_item", SupportLang::Rust);
        let first_impl = root
            .root()
            .find(impl_matcher)
            .expect("Should find an impl block");

        // Find a function_item inside the impl body
        let fn_in_impl = first_impl.find(KindMatcher::new("function_item", SupportLang::Rust));
        assert!(
            fn_in_impl.is_some(),
            "Should find a function inside impl block"
        );

        let inner_fn = fn_in_impl.unwrap();
        let name = inner_fn
            .field("name")
            .map(|n| n.text().to_string())
            .unwrap_or_default();

        // Walk up via parent()
        let parent = inner_fn.parent();
        assert!(parent.is_some(), "Function in impl should have a parent");
        println!(
            "FINDING: fn {} parent kind = {:?}",
            name,
            parent.map(|p| p.kind().to_string())
        );

        // Walk up via ancestors()
        let ancestors: Vec<String> = inner_fn.ancestors().map(|a| a.kind().to_string()).collect();
        println!("FINDING: fn {} ancestors = {:?}", name, ancestors);
        assert!(
            ancestors.iter().any(|a| a == "impl_item"),
            "Ancestors should include impl_item"
        );
        assert!(
            ancestors.iter().any(|a| a == "source_file"),
            "Ancestors should include source_file"
        );
    }

    // =========================================================================
    // Section 5: Position & Text Extraction
    // =========================================================================

    #[test]
    fn spike_position_extraction() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        let fn_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let first_fn = root
            .root()
            .find(fn_matcher)
            .expect("Should find a function");

        let start = first_fn.start_pos();
        let end = first_fn.end_pos();

        // NOTE: Position::column() takes a &Node argument (O(n) operation).
        // This is different from what we assumed in design docs.
        // Position::line() is zero-arg and O(1).
        println!(
            "FINDING: Position API — start_pos().line()={}, start_pos().column(&node)={}, \
             end_pos().line()={}, end_pos().column(&node)={}",
            start.line(),
            start.column(&first_fn),
            end.line(),
            end.column(&first_fn)
        );

        // Position should be zero-based (ast-grep docs say so)
        // The first function in sample.rs (`pub async fn process...`) starts after
        // 2 lines of doc comments, so line should be >= 2 (zero-based)
        assert!(
            start.line() >= 2,
            "First function should start at line >= 2 (zero-based), got {}",
            start.line()
        );

        // End line should be after start line
        assert!(
            end.line() >= start.line(),
            "End line should be >= start line"
        );

        println!(
            "FINDING: For 1-based display, add 1: lines {}-{}",
            start.line() + 1,
            end.line() + 1
        );
    }

    #[test]
    fn spike_text_and_signature_extraction() {
        let source = include_str!("../tests/fixtures/sample.rs");
        let root = SupportLang::Rust.ast_grep(source);

        let fn_matcher = KindMatcher::new("function_item", SupportLang::Rust);
        let first_fn = root
            .root()
            .find(fn_matcher)
            .expect("Should find a function");

        // text() returns Cow<str> with full source including body
        let full_text = first_fn.text();
        assert!(
            full_text.contains('{'),
            "Full text should contain function body"
        );
        println!(
            "FINDING: text() returns Cow<str>, len={}, starts with: {:?}",
            full_text.len(),
            &full_text[..full_text.len().min(80)]
        );

        // Signature extraction: everything before first { or ;
        // This is the klaw pattern (text-based, not AST-based)
        let text = full_text.to_string();
        let brace = text.find('{');
        let semi = text.find(';');
        let end = match (brace, semi) {
            (Some(b), Some(s)) => b.min(s),
            (Some(b), None) => b,
            (None, Some(s)) => s,
            (None, None) => text.len(),
        };
        let signature = text[..end].trim();
        println!("FINDING: Extracted signature: {:?}", signature);
        assert!(signature.contains("fn"), "Signature should contain 'fn'");
        assert!(
            !signature.contains('{'),
            "Signature should not contain body"
        );

        // kind() also returns Cow<str>
        let kind = first_fn.kind();
        assert_eq!(kind.as_ref(), "function_item");
        println!("FINDING: kind() returns Cow<str>: {:?}", kind);
    }

    // =========================================================================
    // Section 6: Pattern Strictness
    // =========================================================================

    #[test]
    fn spike_pattern_strictness_modes() {
        // Test that default "smart" strictness matches `pub fn` with pattern `fn $NAME() {}`
        // Smart mode: all nodes in pattern must match, but unnamed nodes in target are skipped
        let source = "fn foo() {} pub fn bar() {} pub async fn baz() {}";
        let root = SupportLang::Rust.ast_grep(source);

        // Pattern with no pub/async — smart mode should still match all three
        let matches: Vec<_> = root.root().find_all("fn $NAME() { $$$ }").collect();
        let names: Vec<String> = matches
            .iter()
            .filter_map(|m| m.get_env().get_match("NAME").map(|n| n.text().to_string()))
            .collect();

        println!(
            "FINDING: Pattern 'fn $NAME() {{}}' with smart strictness matched: {:?}",
            names
        );
        println!(
            "FINDING: Smart strictness matches {} out of 3 functions",
            matches.len()
        );

        // Even if not all three match, document what happens — this is the key
        // finding about how pattern strictness works for our extraction strategy.
        // If `pub fn bar()` doesn't match `fn $NAME()`, we need kind-based matching
        // as primary strategy (which is what klaw does).
        if matches.len() < 3 {
            println!(
                "FINDING: Smart strictness does NOT match all variants. \
                 Phase 3 should use KindMatcher as primary, patterns as secondary."
            );
        }

        // MatchStrictness enum exists but configuring it from the Rust API
        // requires PatternBuilder or similar. Document whether this is possible.
        // For now, just verify the enum exists at compile time.
        let _smart = ast_grep_core::MatchStrictness::Smart;
        let _ast = ast_grep_core::MatchStrictness::Ast;
        let _cst = ast_grep_core::MatchStrictness::Cst;
        let _relaxed = ast_grep_core::MatchStrictness::Relaxed;
        let _sig = ast_grep_core::MatchStrictness::Signature;
        println!(
            "FINDING: MatchStrictness enum has 5 variants (Cst, Smart, Ast, Relaxed, Signature)"
        );
    }

    // =========================================================================
    // Section 7: Grammar Coverage & Fallback
    // =========================================================================

    #[test]
    fn spike_builtin_grammar_coverage() {
        // Verify all 26 SupportLang variants can parse minimal source
        let cases: &[(&str, SupportLang)] = &[
            ("fn f() {}", SupportLang::Rust),
            ("def f(): pass", SupportLang::Python),
            ("function f() {}", SupportLang::TypeScript),
            ("function f() {}", SupportLang::JavaScript),
            ("const x = <div/>;", SupportLang::Tsx),
            ("package main\nfunc f() {}", SupportLang::Go),
            ("def f, do: :ok", SupportLang::Elixir),
            ("#!/bin/bash\nf() { :; }", SupportLang::Bash),
            ("int f() { return 0; }", SupportLang::C),
            ("int f() { return 0; }", SupportLang::Cpp),
            ("class C { void F() {} }", SupportLang::CSharp),
            ("body { color: red; }", SupportLang::Css),
            ("f :: Int -> Int\nf x = x", SupportLang::Haskell),
            ("resource \"null\" \"x\" {}", SupportLang::Hcl),
            ("<html><body></body></html>", SupportLang::Html),
            ("class C { void f() {} }", SupportLang::Java),
            ("{\"key\": \"value\"}", SupportLang::Json),
            ("fun f(): Int = 1", SupportLang::Kotlin),
            ("function f() end", SupportLang::Lua),
            ("{ x = 1; }", SupportLang::Nix),
            ("<?php function f() {}", SupportLang::Php),
            ("def f; end", SupportLang::Ruby),
            ("object O { def f: Int = 1 }", SupportLang::Scala),
            ("contract C { function f() {} }", SupportLang::Solidity),
            ("func f() -> Int { return 1 }", SupportLang::Swift),
            ("key: value", SupportLang::Yaml),
        ];

        let mut failures = Vec::new();
        for (source, lang) in cases {
            let root = lang.ast_grep(source);
            let node = root.root();
            let children: Vec<_> = node.children().collect();
            if children.is_empty() {
                failures.push(format!("{:?}", lang));
            }
        }

        if !failures.is_empty() {
            println!(
                "WARNING: These languages parsed with no children: {:?}",
                failures
            );
        }
        println!(
            "FINDING: {}/{} built-in grammars parsed successfully",
            cases.len() - failures.len(),
            cases.len()
        );
        assert!(
            failures.is_empty(),
            "All 26 built-in grammars should parse. Failures: {:?}",
            failures
        );
    }

    #[test]
    fn spike_raw_tree_sitter_fallback() {
        // Validate that raw tree-sitter works as a fallback path.
        // tree-sitter is a transitive dependency via ast-grep-language.
        //
        // We test by using ast-grep-language's LanguageExt::get_ts_language()
        // to get the tree_sitter::Language, then using tree-sitter directly.
        let lang = SupportLang::Rust;
        let ts_lang = lang.get_ts_language();

        // Create a tree-sitter Parser directly
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&ts_lang)
            .expect("Should set Rust language on tree-sitter parser");

        let source = b"fn hello() {} fn world() {}";
        let tree = parser
            .parse(&source[..], None)
            .expect("Should parse Rust source");
        let root = tree.root_node();
        assert_eq!(root.kind(), "source_file");

        // Use tree-sitter Query (S-expression pattern) — the alternative to ast-grep patterns
        let query = tree_sitter::Query::new(
            &ts_lang,
            "(function_item name: (identifier) @fn_name) @fn_def",
        )
        .expect("Query should compile");

        let mut cursor = tree_sitter::QueryCursor::new();

        // tree-sitter 0.26: QueryMatches uses StreamingIterator, not std Iterator.
        // Must import the trait and use .next() which returns Option<&Item>.
        use tree_sitter::StreamingIterator;

        let fn_name_idx = query
            .capture_index_for_name("fn_name")
            .expect("fn_name capture should exist");

        let mut names: Vec<String> = Vec::new();
        let mut match_count = 0u32;
        {
            let mut matches = cursor.matches(&query, root, source.as_slice());
            while let Some(m) = matches.next() {
                match_count += 1;
                for capture in m.captures {
                    if capture.index == fn_name_idx {
                        if let Ok(text) = capture.node.utf8_text(source) {
                            names.push(text.to_string());
                        }
                    }
                }
            }
        }

        assert_eq!(
            match_count, 2,
            "Should find 2 functions via tree-sitter query"
        );
        assert_eq!(names, vec!["hello".to_string(), "world".to_string()]);
        println!(
            "FINDING: Raw tree-sitter fallback works. Query found functions: {:?}",
            names
        );
        println!(
            "FINDING: tree-sitter Node.kind() returns &'static str, \
             ast-grep Node.kind() returns Cow<str>"
        );
        println!(
            "FINDING: tree-sitter needs source bytes for text (node.utf8_text(source)), \
             ast-grep stores source internally (node.text())"
        );
    }
}
