use ast_grep_core::Node;
use std::collections::HashSet;

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
    let mut seen = HashSet::new();
    for child in body.children() {
        match child.kind().as_ref() {
            "method_definition" | "abstract_method_signature" | "abstract_method_definition" => {
                if let Some(item) =
                    build_ts_class_callable_member(&child, &owner_name, &visibility, &mut seen)
                {
                    items.push(item);
                }
            }
            "public_field_definition" => {
                if let Some(item) =
                    build_ts_class_field_member(&child, &owner_name, &visibility, &mut seen)
                {
                    items.push(item);
                }
            }
            "index_signature" => {
                if let Some(item) =
                    build_ts_class_indexer_member(&child, &owner_name, &visibility, &mut seen)
                {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    items
}

fn build_ts_class_callable_member<D: ast_grep_core::Doc>(
    child: &Node<D>,
    owner_name: &str,
    visibility: &Visibility,
    seen: &mut HashSet<String>,
) -> Option<ParsedItem> {
    let name = child.field("name").map(|n| n.text().to_string())?;
    let is_accessor = child
        .children()
        .any(|c| c.kind().as_ref() == "get" || c.kind().as_ref() == "set");
    let kind = if is_accessor {
        SymbolKind::Property
    } else if name == "constructor" {
        SymbolKind::Constructor
    } else {
        SymbolKind::Method
    };
    let dedupe_key = if kind == SymbolKind::Property {
        format!("property:{owner_name}:{name}")
    } else {
        format!(
            "{}:{owner_name}:{name}:{}",
            kind,
            child.start_pos().line() as u32 + 1
        )
    };
    if !seen.insert(dedupe_key) {
        return None;
    }

    let metadata = SymbolMetadata {
        owner_name: Some(owner_name.to_string()),
        owner_kind: Some(SymbolKind::Class),
        is_static_member: child.children().any(|c| c.kind().as_ref() == "static"),
        ..Default::default()
    };

    Some(ParsedItem {
        kind,
        name,
        signature: helpers::extract_signature(child),
        source: helpers::extract_source(child, 30),
        doc_comment: String::new(),
        start_line: child.start_pos().line() as u32 + 1,
        end_line: child.end_pos().line() as u32 + 1,
        visibility: visibility.clone(),
        metadata,
    })
}

fn build_ts_class_field_member<D: ast_grep_core::Doc>(
    child: &Node<D>,
    owner_name: &str,
    visibility: &Visibility,
    seen: &mut HashSet<String>,
) -> Option<ParsedItem> {
    let name = child.field("name").map(|n| n.text().to_string())?;
    let kind = if is_event_like_member(&name, child) {
        SymbolKind::Event
    } else if child.children().any(|c| c.kind().as_ref() == "readonly") {
        SymbolKind::Property
    } else {
        SymbolKind::Field
    };
    let dedupe_key = format!("{kind}:{owner_name}:{name}");
    if !seen.insert(dedupe_key) {
        return None;
    }

    let metadata = SymbolMetadata {
        owner_name: Some(owner_name.to_string()),
        owner_kind: Some(SymbolKind::Class),
        is_static_member: child.children().any(|c| c.kind().as_ref() == "static"),
        ..Default::default()
    };

    Some(ParsedItem {
        kind,
        name,
        signature: helpers::extract_signature(child),
        source: helpers::extract_source(child, 20),
        doc_comment: String::new(),
        start_line: child.start_pos().line() as u32 + 1,
        end_line: child.end_pos().line() as u32 + 1,
        visibility: visibility.clone(),
        metadata,
    })
}

fn build_ts_class_indexer_member<D: ast_grep_core::Doc>(
    child: &Node<D>,
    owner_name: &str,
    visibility: &Visibility,
    seen: &mut HashSet<String>,
) -> Option<ParsedItem> {
    let dedupe_key = format!("indexer:{owner_name}");
    if !seen.insert(dedupe_key) {
        return None;
    }

    let metadata = SymbolMetadata {
        owner_name: Some(owner_name.to_string()),
        owner_kind: Some(SymbolKind::Class),
        ..Default::default()
    };

    Some(ParsedItem {
        kind: SymbolKind::Indexer,
        name: format!("{owner_name}[]"),
        signature: helpers::extract_signature(child),
        source: helpers::extract_source(child, 10),
        doc_comment: String::new(),
        start_line: child.start_pos().line() as u32 + 1,
        end_line: child.end_pos().line() as u32 + 1,
        visibility: visibility.clone(),
        metadata,
    })
}

fn is_event_like_member<D: ast_grep_core::Doc>(name: &str, node: &Node<D>) -> bool {
    if name.len() > 2
        && name.starts_with("on")
        && name.chars().nth(2).is_some_and(|c| c.is_ascii_uppercase())
    {
        return true;
    }

    node.children()
        .any(|child| child.kind().as_ref() == "type_annotation" && child.text().contains("Event"))
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
