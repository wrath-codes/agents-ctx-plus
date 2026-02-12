#![allow(
    clippy::field_reassign_with_default,
    clippy::uninlined_format_args,
    clippy::useless_let_if_seq
)]

use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, TypeScriptMetadataExt, Visibility};

use super::super::ts_helpers::{extract_jsdoc_before, parse_jsdoc_sections};

// ── class_declaration / abstract_class_declaration ─────────────────

pub fn process_class<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
    is_default: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    let is_abstract = node.kind().as_ref() == "abstract_class_declaration";

    let extends = extract_class_heritage(node, "extends");
    let implements = extract_class_heritage(node, "implements");
    let (methods, fields) = extract_class_members(node);

    let is_error_type =
        helpers::is_error_type_by_name(&name) || extends.iter().any(|e| e == "Error");

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_exported {
        metadata.mark_exported();
    }
    if is_default {
        metadata.mark_default_export();
    }
    metadata.set_type_parameters(type_params);
    metadata.set_base_classes(extends);
    metadata.set_implements(implements);
    metadata.set_methods(methods);
    metadata.set_fields(fields);
    if is_error_type {
        metadata.mark_error_type();
    }
    if is_abstract {
        metadata.mark_unsafe();
    }
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Class,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

pub fn process_class_members<D: ast_grep_core::Doc>(
    node: &Node<D>,
    is_exported: bool,
) -> Vec<ParsedItem> {
    let Some(owner_name) = node.field("name").map(|n| n.text().to_string()) else {
        return Vec::new();
    };

    let Some(body) = node.field("body") else {
        return Vec::new();
    };

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut items = Vec::new();
    for child in body.children() {
        match child.kind().as_ref() {
            "method_definition" | "abstract_method_signature" | "abstract_method_definition" => {
                if let Some(name_node) = child.field("name") {
                    let name = name_node.text().to_string();
                    let mut kind = if name == "constructor" {
                        SymbolKind::Constructor
                    } else {
                        SymbolKind::Method
                    };
                    if child
                        .children()
                        .any(|c| c.kind().as_ref() == "get" || c.kind().as_ref() == "set")
                    {
                        kind = SymbolKind::Property;
                    }

                    let mut metadata = SymbolMetadata::default();
                    metadata.owner_name = Some(owner_name.clone());
                    metadata.owner_kind = Some(SymbolKind::Class);
                    metadata.is_static_member =
                        child.children().any(|c| c.kind().as_ref() == "static");

                    items.push(ParsedItem {
                        kind,
                        name,
                        signature: helpers::extract_signature(&child),
                        source: helpers::extract_source(&child, 30),
                        doc_comment: String::new(),
                        start_line: child.start_pos().line() as u32 + 1,
                        end_line: child.end_pos().line() as u32 + 1,
                        visibility: visibility.clone(),
                        metadata,
                    });
                }
            }
            "public_field_definition" => {
                if let Some(name_node) = child.field("name") {
                    let name = name_node.text().to_string();
                    let mut metadata = SymbolMetadata::default();
                    metadata.owner_name = Some(owner_name.clone());
                    metadata.owner_kind = Some(SymbolKind::Class);
                    metadata.is_static_member =
                        child.children().any(|c| c.kind().as_ref() == "static");

                    let kind = if child.children().any(|c| c.kind().as_ref() == "readonly") {
                        SymbolKind::Property
                    } else {
                        SymbolKind::Field
                    };

                    items.push(ParsedItem {
                        kind,
                        name,
                        signature: helpers::extract_signature(&child),
                        source: helpers::extract_source(&child, 20),
                        doc_comment: String::new(),
                        start_line: child.start_pos().line() as u32 + 1,
                        end_line: child.end_pos().line() as u32 + 1,
                        visibility: visibility.clone(),
                        metadata,
                    });
                }
            }
            "index_signature" => {
                let mut metadata = SymbolMetadata::default();
                metadata.owner_name = Some(owner_name.clone());
                metadata.owner_kind = Some(SymbolKind::Class);

                items.push(ParsedItem {
                    kind: SymbolKind::Indexer,
                    name: format!("{}[]", owner_name),
                    signature: helpers::extract_signature(&child),
                    source: helpers::extract_source(&child, 10),
                    doc_comment: String::new(),
                    start_line: child.start_pos().line() as u32 + 1,
                    end_line: child.end_pos().line() as u32 + 1,
                    visibility: visibility.clone(),
                    metadata,
                });
            }
            _ => {}
        }
    }

    items
}

fn extract_class_heritage<D: ast_grep_core::Doc>(node: &Node<D>, clause_kind: &str) -> Vec<String> {
    let target = match clause_kind {
        "extends" => "extends_clause",
        "implements" => "implements_clause",
        _ => return Vec::new(),
    };
    for child in node.children() {
        if child.kind().as_ref() == "class_heritage" {
            for clause in child.children() {
                if clause.kind().as_ref() == target {
                    return clause
                        .children()
                        .filter(|c| {
                            let k = c.kind();
                            k.as_ref() != "extends"
                                && k.as_ref() != "implements"
                                && k.as_ref() != ","
                        })
                        .map(|c| c.text().to_string())
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

fn extract_class_members<D: ast_grep_core::Doc>(node: &Node<D>) -> (Vec<String>, Vec<String>) {
    let mut methods = Vec::new();
    let mut fields = Vec::new();

    let Some(body) = node.field("body") else {
        return (methods, fields);
    };

    for child in body.children() {
        let k = child.kind();
        match k.as_ref() {
            "method_definition" => {
                if let Some(name) = child.field("name").map(|n| n.text().to_string()) {
                    methods.push(name);
                }
            }
            "public_field_definition" => {
                if let Some(name) = child.field("name").map(|n| n.text().to_string()) {
                    fields.push(name);
                }
            }
            "abstract_method_definition" | "abstract_method_signature" => {
                if let Some(name) = child.field("name").map(|n| n.text().to_string()) {
                    methods.push(name);
                }
            }
            _ => {}
        }
    }
    (methods, fields)
}
