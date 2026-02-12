//! Type definitions (typedef struct/enum/union/function pointer), top-level
//! struct/union/enum processing, and field/variant extraction.

use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::declarations::{extract_function_pointer_name, has_function_declarator_descendant};
use super::{extract_signature, extract_source_limited};

// ── Type definition processing ─────────────────────────────────────

pub(super) fn process_type_definition<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Find the typedef name (type_identifier at the end)
    let name = children
        .iter()
        .filter(|c| c.kind().as_ref() == "type_identifier")
        .last()
        .map_or_else(
            || {
                // Fallback: could be a primitive_type for things like `typedef unsigned long uint64_t`
                children
                    .iter()
                    .filter(|c| c.kind().as_ref() == "primitive_type")
                    .last()
                    .map_or_else(String::new, |n| n.text().to_string())
            },
            |n| n.text().to_string(),
        );

    if name.is_empty() {
        return;
    }

    // Check if this is a typedef struct or typedef enum
    let has_struct = children
        .iter()
        .any(|c| c.kind().as_ref() == "struct_specifier");
    let has_enum = children
        .iter()
        .any(|c| c.kind().as_ref() == "enum_specifier");
    let has_union = children
        .iter()
        .any(|c| c.kind().as_ref() == "union_specifier");
    let has_func_decl = children
        .iter()
        .any(|c| c.kind().as_ref() == "function_declarator")
        || children.iter().any(|c| {
            c.kind().as_ref() == "pointer_declarator" && has_function_declarator_descendant(c)
        });

    if has_func_decl {
        // For function pointer typedefs the name is nested deep inside
        // function_declarator > parenthesized_declarator > pointer_declarator > type_identifier.
        let fp_name = extract_function_pointer_name(node);
        let fp_name = if fp_name.is_empty() { &name } else { &fp_name };
        process_typedef_function_pointer(node, items, doc_comment, fp_name);
    } else if has_struct {
        // Without a body this is just an alias (e.g. `typedef struct Point Point2D;`)
        let has_body = specifier_has_body(&children, "struct_specifier", "field_declaration_list");
        if has_body {
            process_typedef_struct(node, items, doc_comment, &children, &name);
        } else {
            push_simple_typedef_alias(node, items, doc_comment, name);
        }
    } else if has_enum {
        let has_body = specifier_has_body(&children, "enum_specifier", "enumerator_list");
        if has_body {
            process_typedef_enum(node, items, doc_comment, &children, &name);
        } else {
            push_simple_typedef_alias(node, items, doc_comment, name);
        }
    } else if has_union {
        let has_body = specifier_has_body(&children, "union_specifier", "field_declaration_list");
        if has_body {
            process_typedef_union(node, items, doc_comment, &children, &name);
        } else {
            push_simple_typedef_alias(node, items, doc_comment, name);
        }
    } else {
        push_simple_typedef_alias(node, items, doc_comment, name);
    }
}

/// Check whether a specifier child (struct/union/enum) contains a body node.
fn specifier_has_body<D: ast_grep_core::Doc>(
    children: &[Node<D>],
    specifier_kind: &str,
    body_kind: &str,
) -> bool {
    children
        .iter()
        .find(|c| c.kind().as_ref() == specifier_kind)
        .is_some_and(|s| s.children().any(|c| c.kind().as_ref() == body_kind))
}

/// Emit a simple `TypeAlias` item for a typedef without a body.
fn push_simple_typedef_alias<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    name: String,
) {
    items.push(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["typedef".to_string()],
            ..Default::default()
        },
    });
}

fn process_typedef_struct<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    name: &str,
) {
    let fields = children
        .iter()
        .find(|c| c.kind().as_ref() == "struct_specifier")
        .map_or_else(Vec::new, |s| extract_struct_fields(s));

    items.push(ParsedItem {
        kind: SymbolKind::Struct,
        name: name.to_string(),
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            fields,
            attributes: vec!["typedef".to_string()],
            ..Default::default()
        },
    });
}

fn process_typedef_enum<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    name: &str,
) {
    let variants = children
        .iter()
        .find(|c| c.kind().as_ref() == "enum_specifier")
        .map_or_else(Vec::new, |e| extract_enum_variants(e));

    items.push(ParsedItem {
        kind: SymbolKind::Enum,
        name: name.to_string(),
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            variants,
            attributes: vec!["typedef".to_string()],
            ..Default::default()
        },
    });
}

fn process_typedef_union<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    children: &[Node<D>],
    name: &str,
) {
    let fields = children
        .iter()
        .find(|c| c.kind().as_ref() == "union_specifier")
        .map_or_else(Vec::new, |u| extract_struct_fields(u));

    items.push(ParsedItem {
        kind: SymbolKind::Union,
        name: name.to_string(),
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            fields,
            attributes: vec!["typedef".to_string()],
            ..Default::default()
        },
    });
}

fn process_typedef_function_pointer<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
    name: &str,
) {
    items.push(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name: name.to_string(),
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            attributes: vec!["typedef".to_string(), "function_pointer".to_string()],
            ..Default::default()
        },
    });
}

// ── Top-level struct/union/enum processing ─────────────────────────

pub(super) fn process_top_level_struct<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    // Check if it has a body (field_declaration_list) or is a forward declaration
    let has_body = node
        .children()
        .any(|c| c.kind().as_ref() == "field_declaration_list");

    if has_body {
        let fields = extract_struct_fields(node);
        items.push(ParsedItem {
            kind: SymbolKind::Struct,
            name,
            signature: extract_signature(node),
            source: extract_source_limited(node, 30),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                fields,
                ..Default::default()
            },
        });
    } else {
        // Forward declaration: struct Foo;
        items.push(ParsedItem {
            kind: SymbolKind::Struct,
            name,
            signature: format!(
                "struct {}",
                node.text().as_ref().trim_end_matches(';').trim()
            ),
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                attributes: vec!["forward_declaration".to_string()],
                ..Default::default()
            },
        });
    }
}

pub(super) fn process_top_level_union<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    let fields = extract_struct_fields(node);

    items.push(ParsedItem {
        kind: SymbolKind::Union,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            fields,
            ..Default::default()
        },
    });
}

pub(super) fn process_top_level_enum<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let name = node
        .children()
        .find(|c| c.kind().as_ref() == "type_identifier")
        .map_or_else(String::new, |n| n.text().to_string());

    if name.is_empty() {
        return;
    }

    let has_body = node
        .children()
        .any(|c| c.kind().as_ref() == "enumerator_list");

    if has_body {
        let variants = extract_enum_variants(node);
        items.push(ParsedItem {
            kind: SymbolKind::Enum,
            name,
            signature: extract_signature(node),
            source: extract_source_limited(node, 30),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                variants,
                ..Default::default()
            },
        });
    } else {
        // Forward declaration: enum Foo;
        items.push(ParsedItem {
            kind: SymbolKind::Enum,
            name,
            signature: format!("enum {}", node.text().as_ref().trim_end_matches(';').trim()),
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata: SymbolMetadata {
                attributes: vec!["forward_declaration".to_string()],
                ..Default::default()
            },
        });
    }
}

// ── Field / variant extraction ─────────────────────────────────────

/// Extract field names from a struct or union.
pub(super) fn extract_struct_fields<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut fields = Vec::new();
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "field_declaration_list" {
            let field_children: Vec<_> = child.children().collect();
            for fc in &field_children {
                if fc.kind().as_ref() == "field_declaration" {
                    // Field name may be nested inside pointer_declarator or
                    // array_declarator, so search recursively.
                    if let Some(field_name) = find_field_identifier(fc) {
                        // Check for bit field
                        let has_bitfield = fc
                            .children()
                            .any(|c| c.kind().as_ref() == "bitfield_clause");

                        if has_bitfield {
                            fields.push(format!("{field_name} (bitfield)"));
                        } else {
                            fields.push(field_name);
                        }
                    } else {
                        // Anonymous struct/union inside a field declaration
                        let fc_children: Vec<_> = fc.children().collect();
                        if fc_children
                            .iter()
                            .any(|c| c.kind().as_ref() == "struct_specifier")
                        {
                            fields.push("(anonymous struct)".to_string());
                        } else if fc_children
                            .iter()
                            .any(|c| c.kind().as_ref() == "union_specifier")
                        {
                            fields.push("(anonymous union)".to_string());
                        }
                    }
                }
            }
        }
    }
    fields
}

/// Recursively find a `field_identifier` inside a node.
///
/// Stops at nested `struct_specifier`, `union_specifier`, and
/// `enum_specifier` boundaries to avoid descending into anonymous
/// aggregate members.
fn find_field_identifier<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "field_identifier" {
            return Some(child.text().to_string());
        }
        // Do not descend into nested aggregate types — they belong
        // to anonymous struct/union members handled separately.
        let k = child.kind();
        if matches!(
            k.as_ref(),
            "struct_specifier" | "union_specifier" | "enum_specifier"
        ) {
            continue;
        }
        if let Some(name) = find_field_identifier(child) {
            return Some(name);
        }
    }
    None
}

/// Extract variant names from an enum.
pub(super) fn extract_enum_variants<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut variants = Vec::new();
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "enumerator_list" {
            let list_children: Vec<_> = child.children().collect();
            for lc in &list_children {
                if lc.kind().as_ref() == "enumerator"
                    && let Some(id) = lc.children().find(|c| c.kind().as_ref() == "identifier")
                {
                    variants.push(id.text().to_string());
                }
            }
        }
    }
    variants
}
