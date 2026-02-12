use ast_grep_core::Node;
use std::collections::HashSet;

use crate::extractors::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, TypeScriptMetadataExt, Visibility};

use super::super::ts_helpers::{
    extract_jsdoc_before, extract_ts_parameters, extract_ts_return_type, parse_jsdoc_sections,
};

// ── interface_declaration ──────────────────────────────────────────

pub fn process_interface<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    let extends = extract_interface_heritage(node);
    let members = extract_interface_members(node);

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_exported {
        metadata.mark_exported();
    }
    metadata.set_type_parameters(type_params);
    metadata.set_base_classes(extends);
    metadata.set_methods(members);
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Interface,
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

pub fn process_interface_members<D: ast_grep_core::Doc>(
    node: &Node<D>,
    is_exported: bool,
) -> Vec<ParsedItem> {
    let Some(owner_name) = node.field("name").map(|n| n.text().to_string()) else {
        return Vec::new();
    };

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for child in node.children() {
        if child.kind().as_ref() != "interface_body" && child.kind().as_ref() != "object_type" {
            continue;
        }

        for member in child.children() {
            match member.kind().as_ref() {
                "method_signature" => {
                    if let Some(item) =
                        build_interface_method_member(&member, &owner_name, &visibility, &mut seen)
                    {
                        items.push(item);
                    }
                }
                "property_signature" => {
                    if let Some(item) = build_interface_property_member(
                        &member,
                        &owner_name,
                        &visibility,
                        &mut seen,
                    ) {
                        items.push(item);
                    }
                }
                "index_signature" => {
                    if let Some(item) =
                        build_interface_indexer_member(&member, &owner_name, &visibility, &mut seen)
                    {
                        items.push(item);
                    }
                }
                _ => {}
            }
        }
    }

    items
}

fn build_interface_method_member<D: ast_grep_core::Doc>(
    member: &Node<D>,
    owner_name: &str,
    visibility: &Visibility,
    seen: &mut HashSet<String>,
) -> Option<ParsedItem> {
    let member_name = member.field("name").map(|n| n.text().to_string())?;
    let dedupe_key = format!("method:{owner_name}:{member_name}");
    if !seen.insert(dedupe_key) {
        return None;
    }

    let metadata = SymbolMetadata {
        owner_name: Some(owner_name.to_string()),
        owner_kind: Some(SymbolKind::Interface),
        return_type: extract_ts_return_type(member),
        parameters: extract_ts_parameters(member),
        ..Default::default()
    };

    Some(ParsedItem {
        kind: SymbolKind::Method,
        name: format!("{owner_name}::{member_name}"),
        signature: helpers::extract_signature(member),
        source: helpers::extract_source(member, 10),
        doc_comment: String::new(),
        start_line: member.start_pos().line() as u32 + 1,
        end_line: member.end_pos().line() as u32 + 1,
        visibility: visibility.clone(),
        metadata,
    })
}

fn build_interface_property_member<D: ast_grep_core::Doc>(
    member: &Node<D>,
    owner_name: &str,
    visibility: &Visibility,
    seen: &mut HashSet<String>,
) -> Option<ParsedItem> {
    let member_name = member.field("name").map(|n| n.text().to_string())?;
    let kind = if is_event_like_interface_member(&member_name, member) {
        SymbolKind::Event
    } else {
        SymbolKind::Property
    };
    let dedupe_key = format!("{kind}:{owner_name}:{member_name}");
    if !seen.insert(dedupe_key) {
        return None;
    }

    let metadata = SymbolMetadata {
        owner_name: Some(owner_name.to_string()),
        owner_kind: Some(SymbolKind::Interface),
        return_type: member
            .children()
            .find(|c| c.kind().as_ref() == "type_annotation")
            .map(|ta| {
                ta.text()
                    .to_string()
                    .trim_start_matches(':')
                    .trim()
                    .to_string()
            }),
        ..Default::default()
    };

    Some(ParsedItem {
        kind,
        name: format!("{owner_name}::{member_name}"),
        signature: helpers::extract_signature(member),
        source: helpers::extract_source(member, 10),
        doc_comment: String::new(),
        start_line: member.start_pos().line() as u32 + 1,
        end_line: member.end_pos().line() as u32 + 1,
        visibility: visibility.clone(),
        metadata,
    })
}

fn build_interface_indexer_member<D: ast_grep_core::Doc>(
    member: &Node<D>,
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
        owner_kind: Some(SymbolKind::Interface),
        ..Default::default()
    };

    Some(ParsedItem {
        kind: SymbolKind::Indexer,
        name: format!("{owner_name}[]"),
        signature: helpers::extract_signature(member),
        source: helpers::extract_source(member, 10),
        doc_comment: String::new(),
        start_line: member.start_pos().line() as u32 + 1,
        end_line: member.end_pos().line() as u32 + 1,
        visibility: visibility.clone(),
        metadata,
    })
}

fn extract_interface_heritage<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    for child in node.children() {
        if child.kind().as_ref() == "extends_type_clause" {
            return child
                .children()
                .filter(|c| c.kind().as_ref() != "extends" && c.kind().as_ref() != ",")
                .map(|c| c.text().to_string())
                .collect();
        }
    }
    Vec::new()
}

fn extract_interface_members<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut members = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "interface_body" || child.kind().as_ref() == "object_type" {
            for member in child.children() {
                let mk = member.kind();
                if (mk.as_ref() == "method_signature" || mk.as_ref() == "property_signature")
                    && let Some(name) = member.field("name").map(|n| n.text().to_string())
                {
                    members.push(name);
                }
            }
        }
    }
    members
}

fn is_event_like_interface_member<D: ast_grep_core::Doc>(name: &str, node: &Node<D>) -> bool {
    if name.len() > 2
        && name.starts_with("on")
        && name.chars().nth(2).is_some_and(|c| c.is_ascii_uppercase())
    {
        return true;
    }

    node.children()
        .any(|child| child.kind().as_ref() == "type_annotation" && child.text().contains("Event"))
}

// ── type_alias_declaration ─────────────────────────────────────────

pub fn process_type_alias<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_exported {
        metadata.mark_exported();
    }
    metadata.set_type_parameters(type_params);
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::TypeAlias,
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

// ── enum_declaration ───────────────────────────────────────────────

pub fn process_enum<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let variants = extract_enum_members(node);

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    metadata.set_variants(variants);
    if is_exported {
        metadata.mark_exported();
    }
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Enum,
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

fn extract_enum_members<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut variants = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "enum_body" {
            for member in child.children() {
                if member.kind().as_ref() == "enum_assignment"
                    || member.kind().as_ref() == "property_identifier"
                {
                    let name = member
                        .field("name")
                        .map_or_else(|| member.text().to_string(), |n| n.text().to_string());
                    if !name.is_empty() && name != "," && name != "{" && name != "}" {
                        variants.push(name);
                    }
                }
            }
        }
    }
    variants
}

// ── namespace (internal_module) ─────────────────────────────────────

pub fn process_namespace<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_exported {
        metadata.mark_exported();
    }
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: format!(
            "namespace {}",
            node.field("name")
                .map(|n| n.text().to_string())
                .unwrap_or_default()
        ),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}
