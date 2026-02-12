//! C rich extractor.
//!
//! Extracts structurally significant elements from C source files:
//! functions (definitions and prototypes with static/inline/extern qualifiers),
//! structs (with field extraction, typedef structs), unions, enums (with
//! variant extraction), typedefs (simple, struct, function pointer),
//! global variables, constants, preprocessor directives (`#include`,
//! `#define` object-like and function-like macros, `#ifdef`/`#ifndef`,
//! `#pragma`), function pointers, bit fields, array declarations,
//! forward declarations, and doc comments (`/* */`, `/** */`, `//`).

mod core;
mod declarations;
mod helpers;
mod preproc;
mod typedefs;

use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use crate::types::ParsedItem;

use declarations::{process_declaration, process_function_definition};
use preproc::{
    process_expression_statement, process_preproc_call, process_preproc_def,
    process_preproc_function_def, process_preproc_if, process_preproc_ifdef,
    process_preproc_include,
};
use typedefs::{
    process_top_level_enum, process_top_level_struct, process_top_level_union,
    process_type_definition,
};

/// Extract all significant elements from a C source file.
///
/// Walks the top-level `translation_unit` node collecting functions,
/// structs, unions, enums, typedefs, variables, constants, preprocessor
/// directives, and forward declarations.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
#[allow(clippy::unnecessary_wraps)]
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    source: &str,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let root_node = root.root();
    collect_top_level(&root_node, &mut items, source);
    Ok(items)
}

// ── Top-level node dispatcher ──────────────────────────────────────

fn collect_top_level<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();

    for (idx, child) in children.iter().enumerate() {
        let kind = child.kind();
        match kind.as_ref() {
            "function_definition" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_function_definition(child, items, &doc);
            }
            "declaration" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_declaration(child, items, &doc);
            }
            "type_definition" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_type_definition(child, items, &doc);
            }
            "struct_specifier" => {
                // Top-level `struct Foo { ... };` or `struct Foo;`
                let doc = collect_doc_comment(&children, idx, source);
                process_top_level_struct(child, items, &doc);
            }
            "union_specifier" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_top_level_union(child, items, &doc);
            }
            "enum_specifier" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_top_level_enum(child, items, &doc);
            }
            "preproc_include" => {
                process_preproc_include(child, items);
            }
            "preproc_def" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_preproc_def(child, items, &doc);
            }
            "preproc_function_def" => {
                let doc = collect_doc_comment(&children, idx, source);
                process_preproc_function_def(child, items, &doc);
            }
            "preproc_ifdef" => {
                process_preproc_ifdef(child, items, source);
            }
            "preproc_if" => {
                process_preproc_if(child, items, source);
            }
            "preproc_call" => {
                process_preproc_call(child, items);
            }
            "expression_statement" => {
                // _Static_assert is parsed as expression_statement > call_expression
                let doc = collect_doc_comment(&children, idx, source);
                process_expression_statement(child, items, &doc);
            }
            _ => {} // Skip comments, punctuation, etc.
        }
    }
}

// ── Doc comment collection ─────────────────────────────────────────

/// Collect leading doc comments above a node.
///
/// Walks backward through siblings from `idx`, collecting contiguous
/// `comment` nodes. Supports `//`, `/* */`, and `/** */` styles.
/// Stops at any non-comment node or a blank-line gap.
fn collect_doc_comment<D: ast_grep_core::Doc>(
    siblings: &[Node<D>],
    idx: usize,
    _source: &str,
) -> String {
    let mut comments = Vec::new();
    let target_line = siblings[idx].start_pos().line();

    let mut i = idx;
    while i > 0 {
        i -= 1;
        let sibling = &siblings[i];
        if sibling.kind().as_ref() != "comment" {
            break;
        }

        // Check for line gap — comments must be contiguous
        let comment_end = sibling.end_pos().line();
        let next_start = if i + 1 < idx {
            siblings[i + 1].start_pos().line()
        } else {
            target_line
        };
        if next_start > comment_end + 1 {
            break;
        }

        let text = sibling.text().to_string();
        let stripped = strip_comment(&text);
        if !stripped.is_empty() {
            comments.push(stripped);
        }
    }

    comments.reverse();
    comments.join("\n")
}

/// Strip C comment markers from a comment string.
fn strip_comment(text: &str) -> String {
    let text = text.trim();

    // Single-line: // ...
    if let Some(rest) = text.strip_prefix("//") {
        return rest.trim().to_string();
    }

    // Multi-line: /* ... */ or /** ... */
    let inner = text
        .strip_prefix("/**")
        .or_else(|| text.strip_prefix("/*"))
        .unwrap_or(text);
    let inner = inner.strip_suffix("*/").unwrap_or(inner);

    // Process each line, stripping leading ` * ` decoration
    inner
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            let stripped = trimmed
                .strip_prefix("* ")
                .unwrap_or_else(|| trimmed.strip_prefix('*').unwrap_or(trimmed));
            stripped.trim()
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Source extraction helper ───────────────────────────────────────

/// Extract full source up to `max_lines` lines.
#[allow(clippy::unnecessary_wraps)]
fn extract_source_limited<D: ast_grep_core::Doc>(
    node: &Node<D>,
    max_lines: usize,
) -> Option<String> {
    let text = node.text().to_string();
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        Some(text)
    } else {
        let truncated: String = lines[..max_lines].join("\n");
        Some(format!(
            "{truncated}\n    // ... ({} more lines)",
            lines.len() - max_lines
        ))
    }
}

// ── Signature extraction ───────────────────────────────────────────

/// Extract signature from a node: everything before first `{`, whitespace-normalized.
fn extract_signature<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let text = node.text().to_string();
    let brace = text.find('{');
    let semi = text.find(';');
    let end = match (brace, semi) {
        (Some(b), Some(s)) => b.min(s),
        (Some(b), None) => b,
        (None, Some(s)) => s,
        (None, None) => text.len(),
    };
    let sig = text[..end].trim();
    sig.split_whitespace().collect::<Vec<_>>().join(" ")
}
