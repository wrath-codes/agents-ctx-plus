//! Preprocessor directive processing (#include, #define, #ifdef, #if, #pragma, etc.).

use ast_grep_core::Node;
use std::fmt::Write as _;

use crate::types::{CMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::declarations::{process_declaration, process_function_definition};
use super::typedefs::{
    process_top_level_enum, process_top_level_struct, process_top_level_union,
    process_type_definition,
};
use super::{collect_doc_comment, extract_source_limited};

// ── Include processing ─────────────────────────────────────────────

pub(super) fn process_preproc_include<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
) {
    let children: Vec<_> = node.children().collect();

    // The path is either system_lib_string or string_literal
    let path = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() == "system_lib_string" || k.as_ref() == "string_literal"
        })
        .map_or_else(String::new, |n| n.text().to_string());

    if path.is_empty() {
        return;
    }

    let is_system = children
        .iter()
        .any(|c| c.kind().as_ref() == "system_lib_string");

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("include");
    if is_system {
        metadata.push_attribute("system");
    } else {
        metadata.push_attribute("local");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name: path,
        signature: node.text().to_string().trim().to_string(),
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── Define processing ──────────────────────────────────────────────

pub(super) fn process_preproc_def<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    let has_value = children.iter().any(|c| c.kind().as_ref() == "preproc_arg");

    let value = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_arg")
        .map(|n| n.text().to_string().trim().to_string());

    // Object-like macros with a value are constants; header guard defines without value are macros
    let kind = if has_value {
        SymbolKind::Const
    } else {
        SymbolKind::Macro
    };

    let mut signature = String::new();
    let _ = write!(signature, "#define {name}");
    if let Some(ref v) = value {
        let _ = write!(signature, " {v}");
    }

    items.push(ParsedItem {
        kind,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["define".to_string()],
            ..Default::default()
        },
    });
}

pub(super) fn process_preproc_function_def<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let name = children
        .iter()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    let params = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_params")
        .map_or_else(String::new, |n| n.text().to_string());

    let body = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_arg")
        .map(|n| n.text().to_string().trim().to_string());

    let mut signature = String::new();
    let _ = write!(signature, "#define {name}{params}");
    if let Some(ref b) = body {
        let _ = write!(signature, " {b}");
    }

    // Extract parameter names
    let param_names: Vec<String> = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_params")
        .map_or_else(Vec::new, |p| {
            p.children()
                .filter(|c| c.kind().as_ref() == "identifier" || c.kind().as_ref() == "...")
                .map(|c| c.text().to_string())
                .collect()
        });

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            parameters: param_names,
            attributes: vec!["define".to_string(), "function_like".to_string()],
            ..Default::default()
        },
    });
}

// ── Ifdef / if / elif / else processing ────────────────────────────

pub(super) fn process_preproc_ifdef<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Determine if it's #ifdef or #ifndef
    let is_ifndef = children.iter().any(|c| c.kind().as_ref() == "#ifndef");

    let condition_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if condition_name.is_empty() {
        return;
    }

    let directive = if is_ifndef { "#ifndef" } else { "#ifdef" };

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: condition_name.clone(),
        signature: format!("{directive} {condition_name}"),
        source: extract_source_limited(node, 5),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec![directive.to_string()],
            ..Default::default()
        },
    });

    // Also process any children inside the ifdef block (same dispatch as top-level)
    process_ifdef_children(&children, items, source);
}

pub(super) fn process_preproc_if<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Extract condition: first child that isn't `#if`
    let condition = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() != "#if"
                && k.as_ref() != "#endif"
                && k.as_ref() != "preproc_elif"
                && k.as_ref() != "preproc_else"
                && !matches!(
                    k.as_ref(),
                    "function_definition"
                        | "declaration"
                        | "type_definition"
                        | "struct_specifier"
                        | "union_specifier"
                        | "enum_specifier"
                        | "preproc_include"
                        | "preproc_def"
                        | "preproc_function_def"
                        | "preproc_ifdef"
                        | "preproc_if"
                        | "preproc_call"
                        | "expression_statement"
                        | "comment"
                )
        })
        .map_or_else(String::new, |n| n.text().to_string());

    if !condition.is_empty() {
        items.push(ParsedItem {
            kind: SymbolKind::Macro,
            name: condition.clone(),
            signature: format!("#if {condition}"),
            source: extract_source_limited(node, 5),
            doc_comment: String::new(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                attributes: vec!["#if".to_string()],
                ..Default::default()
            },
        });
    }

    // Process declarations inside the #if block
    process_ifdef_children(&children, items, source);

    // Handle nested preproc_elif and preproc_else
    for child in &children {
        match child.kind().as_ref() {
            "preproc_elif" => process_preproc_elif(child, items, source),
            "preproc_else" => process_preproc_else(child, items, source),
            _ => {}
        }
    }
}

fn process_preproc_elif<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Extract condition (skip the `#elif` keyword itself)
    let condition = children
        .iter()
        .find(|c| {
            let k = c.kind();
            k.as_ref() != "#elif"
                && !matches!(
                    k.as_ref(),
                    "function_definition"
                        | "declaration"
                        | "type_definition"
                        | "struct_specifier"
                        | "union_specifier"
                        | "enum_specifier"
                        | "preproc_include"
                        | "preproc_def"
                        | "preproc_function_def"
                        | "preproc_ifdef"
                        | "preproc_if"
                        | "preproc_call"
                        | "expression_statement"
                        | "comment"
                        | "preproc_else"
                )
        })
        .map_or_else(String::new, |n| n.text().to_string());

    if !condition.is_empty() {
        items.push(ParsedItem {
            kind: SymbolKind::Macro,
            name: condition.clone(),
            signature: format!("#elif {condition}"),
            source: extract_source_limited(node, 5),
            doc_comment: String::new(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                attributes: vec!["#elif".to_string()],
                ..Default::default()
            },
        });
    }

    // Process declarations inside the #elif block
    process_ifdef_children(&children, items, source);

    // Handle nested preproc_else
    for child in &children {
        if child.kind().as_ref() == "preproc_else" {
            process_preproc_else(child, items, source);
        }
    }
}

fn process_preproc_else<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    let children: Vec<_> = node.children().collect();
    process_ifdef_children(&children, items, source);
}

/// Process children inside a `preproc_ifdef` block.
///
/// Uses the same dispatch logic as the top-level walker so that
/// structs, enums, unions, typedefs, etc. inside `#ifndef` guards
/// are correctly extracted.
fn process_ifdef_children<D: ast_grep_core::Doc>(
    children: &[Node<D>],
    items: &mut Vec<ParsedItem>,
    source: &str,
) {
    for (idx, child) in children.iter().enumerate() {
        let kind = child.kind();
        match kind.as_ref() {
            "function_definition" => {
                let doc = collect_doc_comment(children, idx, source);
                process_function_definition(child, items, &doc);
            }
            "declaration" => {
                let doc = collect_doc_comment(children, idx, source);
                process_declaration(child, items, &doc);
            }
            "type_definition" => {
                let doc = collect_doc_comment(children, idx, source);
                process_type_definition(child, items, &doc);
            }
            "struct_specifier" => {
                let doc = collect_doc_comment(children, idx, source);
                process_top_level_struct(child, items, &doc);
            }
            "union_specifier" => {
                let doc = collect_doc_comment(children, idx, source);
                process_top_level_union(child, items, &doc);
            }
            "enum_specifier" => {
                let doc = collect_doc_comment(children, idx, source);
                process_top_level_enum(child, items, &doc);
            }
            "preproc_include" => {
                process_preproc_include(child, items);
            }
            "preproc_def" => {
                let doc = collect_doc_comment(children, idx, source);
                process_preproc_def(child, items, &doc);
            }
            "preproc_function_def" => {
                let doc = collect_doc_comment(children, idx, source);
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
                let doc = collect_doc_comment(children, idx, source);
                process_expression_statement(child, items, &doc);
            }
            _ => {}
        }
    }
}

pub(super) fn process_preproc_call<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
) {
    let children: Vec<_> = node.children().collect();

    let directive = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_directive")
        .map_or_else(String::new, |n| n.text().to_string());

    if directive.is_empty() {
        return;
    }

    let args = children
        .iter()
        .find(|c| c.kind().as_ref() == "preproc_arg")
        .map(|n| n.text().to_string().trim().to_string());

    let name = args.as_deref().unwrap_or(&directive).to_string();

    let mut signature = directive.clone();
    if let Some(ref a) = args {
        let _ = write!(signature, " {a}");
    }

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec![directive],
            ..Default::default()
        },
    });
}

// ── Expression statement processing (_Static_assert) ───────────────

pub(super) fn process_expression_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    // Look for call_expression with _Static_assert
    let children: Vec<_> = node.children().collect();
    let Some(call) = children
        .iter()
        .find(|c| c.kind().as_ref() == "call_expression")
    else {
        return;
    };

    let call_name = call
        .children()
        .find(|c| c.kind().as_ref() == "identifier")
        .map(|n| n.text().to_string());

    if call_name.as_deref() != Some("_Static_assert") {
        return;
    }

    items.push(ParsedItem {
        kind: SymbolKind::Macro,
        name: "_Static_assert".to_string(),
        signature: node
            .text()
            .to_string()
            .trim_end_matches(';')
            .trim()
            .to_string(),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["static_assert".to_string()],
            ..Default::default()
        },
    });
}
