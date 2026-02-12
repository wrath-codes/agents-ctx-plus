#![allow(clippy::field_reassign_with_default, clippy::useless_let_if_seq)]

use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{JavaScriptMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::js_helpers::{extract_js_parameters, extract_jsdoc_before, parse_jsdoc_sections};

// ── class_declaration ──────────────────────────────────────────────

pub fn process_class<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
    is_default: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);

    let extends = extract_class_heritage(node);
    let methods = extract_class_methods(node);

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
    metadata.set_base_classes(extends);
    metadata.set_methods(methods);
    if is_error_type {
        metadata.mark_error_type();
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

    let mut members = Vec::new();
    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    for child in body.children() {
        match child.kind().as_ref() {
            "method_definition" => {
                if let Some(name_node) = child.field("name") {
                    let name = name_node.text().to_string();
                    let mut metadata = SymbolMetadata::default();
                    metadata.owner_name = Some(owner_name.clone());
                    metadata.owner_kind = Some(SymbolKind::Class);
                    metadata.is_static_member =
                        child.children().any(|c| c.kind().as_ref() == "static");
                    metadata.parameters = child
                        .children()
                        .find(|c| c.kind().as_ref() == "formal_parameters")
                        .map(|p| extract_js_parameters(&p))
                        .unwrap_or_default();

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

                    members.push(ParsedItem {
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
            "public_field_definition" | "field_definition" => {
                if let Some(name_node) = child.field("name") {
                    let name = name_node.text().to_string();
                    let mut metadata = SymbolMetadata::default();
                    metadata.owner_name = Some(owner_name.clone());
                    metadata.owner_kind = Some(SymbolKind::Class);
                    metadata.is_static_member =
                        child.children().any(|c| c.kind().as_ref() == "static");

                    members.push(ParsedItem {
                        kind: SymbolKind::Field,
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
            _ => {}
        }
    }

    members
}

fn extract_class_heritage<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    for child in node.children() {
        if child.kind().as_ref() == "class_heritage" {
            // JS: class_heritage → extends + identifier (no extends_clause wrapper)
            return child
                .children()
                .filter(|c| {
                    let k = c.kind();
                    k.as_ref() != "extends" && k.as_ref() != ","
                })
                .map(|c| c.text().to_string())
                .collect();
        }
    }
    Vec::new()
}

fn extract_class_methods<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut methods = Vec::new();
    let Some(body) = node.field("body") else {
        return methods;
    };

    for child in body.children() {
        if child.kind().as_ref() == "method_definition"
            && let Some(name) = child.field("name").map(|n| n.text().to_string())
        {
            methods.push(name);
        }
    }
    methods
}
