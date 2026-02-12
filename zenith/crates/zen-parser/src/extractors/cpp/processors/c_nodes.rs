//! Lightweight C-node handlers for namespace interiors and linkage blocks.
//!
//! These re-implement minimal C-style extraction for nodes found inside
//! namespace blocks and extern "C" blocks (which the top-level C extractor
//! doesn't see).

use ast_grep_core::Node;

use crate::types::{CppMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::helpers::{
    extract_enum_variants, extract_field_names, extract_method_names,
    extract_parameters_from_declarator, extract_return_type_from_children,
    find_identifier_recursive,
};
use super::{extract_signature, extract_source_limited};

pub(super) fn process_c_function_definition<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    else {
        return;
    };

    let name = find_identifier_recursive(func_decl);
    if name.is_empty() {
        return;
    }

    let return_type = extract_return_type_from_children(&children);
    let parameters = extract_parameters_from_declarator(func_decl);

    let mut attrs = Vec::new();
    for child in &children {
        match child.kind().as_ref() {
            "storage_class_specifier" => {
                let t = child.text();
                match t.as_ref() {
                    "static" => attrs.push("static".to_string()),
                    "inline" => attrs.push("inline".to_string()),
                    "extern" => attrs.push("extern".to_string()),
                    _ => {}
                }
            }
            "type_qualifier" => {
                let t = child.text();
                match t.as_ref() {
                    "constexpr" => attrs.push("constexpr".to_string()),
                    "consteval" => attrs.push("consteval".to_string()),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let visibility = if attrs.contains(&"static".to_string()) {
        Visibility::Private
    } else {
        Visibility::Public
    };

    let mut metadata = SymbolMetadata::default();
    metadata.set_return_type(return_type);
    metadata.set_parameters(parameters);
    for attr in attrs {
        metadata.push_attribute(attr);
    }

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    });
}

pub(super) fn process_c_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();

    // Function prototype
    if let Some(func_decl) = children
        .iter()
        .find(|c| c.kind().as_ref() == "function_declarator")
    {
        let name = find_identifier_recursive(func_decl);
        if !name.is_empty() {
            let return_type = extract_return_type_from_children(&children);
            let parameters = extract_parameters_from_declarator(func_decl);
            let mut metadata = SymbolMetadata::default();
            metadata.set_return_type(return_type);
            metadata.set_parameters(parameters);
            metadata.push_attribute("prototype");

            items.push(ParsedItem {
                kind: SymbolKind::Function,
                name,
                signature: extract_signature(node),
                source: Some(node.text().to_string()),
                doc_comment: doc_comment.to_string(),
                start_line: node.start_pos().line() as u32 + 1,
                end_line: node.end_pos().line() as u32 + 1,
                visibility: Visibility::Public,
                metadata,
            });
        }
        return;
    }

    // Variable declaration
    let init_decls: Vec<_> = children
        .iter()
        .filter(|c| c.kind().as_ref() == "init_declarator")
        .collect();
    for init_decl in &init_decls {
        let name = find_identifier_recursive(init_decl);
        if name.is_empty() {
            continue;
        }
        let return_type = extract_return_type_from_children(&children);
        let is_const = children
            .iter()
            .any(|c| c.kind().as_ref() == "type_qualifier" && c.text().as_ref() == "const")
            || children
                .iter()
                .any(|c| c.kind().as_ref() == "type_qualifier" && c.text().as_ref() == "constexpr");

        let kind = if is_const {
            SymbolKind::Const
        } else {
            SymbolKind::Static
        };
        let mut metadata = SymbolMetadata::default();
        metadata.set_return_type(return_type);

        items.push(ParsedItem {
            kind,
            name,
            signature: extract_signature(node),
            source: Some(node.text().to_string()),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata,
        });
    }

    // Plain identifiers
    if init_decls.is_empty() {
        for child in &children {
            if child.kind().as_ref() == "identifier" {
                items.push(ParsedItem {
                    kind: SymbolKind::Static,
                    name: child.text().to_string(),
                    signature: extract_signature(node),
                    source: Some(node.text().to_string()),
                    doc_comment: doc_comment.to_string(),
                    start_line: node.start_pos().line() as u32 + 1,
                    end_line: node.end_pos().line() as u32 + 1,
                    visibility: Visibility::Public,
                    metadata: SymbolMetadata::default(),
                });
            }
        }
    }
}

pub(super) fn process_c_struct<D: ast_grep_core::Doc>(
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
        .any(|c| c.kind().as_ref() == "field_declaration_list");
    if has_body {
        let fields = extract_field_names(node);
        let methods = extract_method_names(node);
        let mut metadata = SymbolMetadata::default();
        metadata.set_fields(fields);
        metadata.set_methods(methods);

        items.push(ParsedItem {
            kind: SymbolKind::Struct,
            name,
            signature: extract_signature(node),
            source: extract_source_limited(node, 30),
            doc_comment: doc_comment.to_string(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata,
        });
    }
}

pub(super) fn process_c_enum<D: ast_grep_core::Doc>(
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
    let is_scoped = node.children().any(|c| c.kind().as_ref() == "class");
    let variants = extract_enum_variants(node);
    let mut metadata = SymbolMetadata::default();
    metadata.set_variants(variants);
    if is_scoped {
        metadata.push_attribute("scoped_enum");
    }
    items.push(ParsedItem {
        kind: SymbolKind::Enum,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

pub(super) fn process_c_typedef<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    doc_comment: &str,
) {
    let children: Vec<_> = node.children().collect();
    let name = children
        .iter()
        .filter(|c| c.kind().as_ref() == "type_identifier")
        .last()
        .map_or_else(String::new, |n| n.text().to_string());
    if name.is_empty() {
        return;
    }
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("typedef");

    items.push(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name,
        signature: extract_signature(node),
        source: Some(node.text().to_string()),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

pub(super) fn process_c_union<D: ast_grep_core::Doc>(
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
    let fields = extract_field_names(node);
    let mut metadata = SymbolMetadata::default();
    metadata.set_fields(fields);

    items.push(ParsedItem {
        kind: SymbolKind::Union,
        name,
        signature: extract_signature(node),
        source: extract_source_limited(node, 30),
        doc_comment: doc_comment.to_string(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}
