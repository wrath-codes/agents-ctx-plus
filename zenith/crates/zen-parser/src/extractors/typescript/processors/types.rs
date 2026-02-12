#![allow(clippy::field_reassign_with_default, clippy::uninlined_format_args)]

use ast_grep_core::Node;

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
    for child in node.children() {
        if child.kind().as_ref() != "interface_body" && child.kind().as_ref() != "object_type" {
            continue;
        }

        for member in child.children() {
            match member.kind().as_ref() {
                "method_signature" => {
                    if let Some(name_node) = member.field("name") {
                        let mut metadata = SymbolMetadata::default();
                        metadata.owner_name = Some(owner_name.clone());
                        metadata.owner_kind = Some(SymbolKind::Interface);
                        metadata.return_type = extract_ts_return_type(&member);
                        metadata.parameters = extract_ts_parameters(&member);

                        items.push(ParsedItem {
                            kind: SymbolKind::Method,
                            name: format!("{}::{}", owner_name, name_node.text()),
                            signature: helpers::extract_signature(&member),
                            source: helpers::extract_source(&member, 10),
                            doc_comment: String::new(),
                            start_line: member.start_pos().line() as u32 + 1,
                            end_line: member.end_pos().line() as u32 + 1,
                            visibility: visibility.clone(),
                            metadata,
                        });
                    }
                }
                "property_signature" => {
                    if let Some(name_node) = member.field("name") {
                        let mut metadata = SymbolMetadata::default();
                        metadata.owner_name = Some(owner_name.clone());
                        metadata.owner_kind = Some(SymbolKind::Interface);
                        metadata.return_type = member
                            .children()
                            .find(|c| c.kind().as_ref() == "type_annotation")
                            .map(|ta| {
                                ta.text()
                                    .to_string()
                                    .trim_start_matches(':')
                                    .trim()
                                    .to_string()
                            });

                        items.push(ParsedItem {
                            kind: SymbolKind::Property,
                            name: format!("{}::{}", owner_name, name_node.text()),
                            signature: helpers::extract_signature(&member),
                            source: helpers::extract_source(&member, 10),
                            doc_comment: String::new(),
                            start_line: member.start_pos().line() as u32 + 1,
                            end_line: member.end_pos().line() as u32 + 1,
                            visibility: visibility.clone(),
                            metadata,
                        });
                    }
                }
                "index_signature" => {
                    let mut metadata = SymbolMetadata::default();
                    metadata.owner_name = Some(owner_name.clone());
                    metadata.owner_kind = Some(SymbolKind::Interface);
                    items.push(ParsedItem {
                        kind: SymbolKind::Indexer,
                        name: format!("{}[]", owner_name),
                        signature: helpers::extract_signature(&member),
                        source: helpers::extract_source(&member, 10),
                        doc_comment: String::new(),
                        start_line: member.start_pos().line() as u32 + 1,
                        end_line: member.end_pos().line() as u32 + 1,
                        visibility: visibility.clone(),
                        metadata,
                    });
                }
                _ => {}
            }
        }
    }

    items
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
