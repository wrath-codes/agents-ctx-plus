use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, TypeScriptMetadataExt, Visibility};

use super::super::ts_helpers::{
    extract_jsdoc_before, extract_ts_parameters, extract_ts_return_type, parse_jsdoc_sections,
};

// ── function_declaration ───────────────────────────────────────────

pub fn process_function<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
    is_default: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let is_async = node.children().any(|c| c.kind().as_ref() == "async");
    let return_type = extract_ts_return_type(node);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_async {
        metadata.mark_async();
    }
    if is_exported {
        metadata.mark_exported();
    }
    if is_default {
        metadata.mark_default_export();
    }
    metadata.set_return_type(return_type);
    metadata.set_type_parameters(type_params);
    metadata.set_parameters(extract_ts_parameters(node));
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Function,
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

// ── function_signature (overloads, declare function) ───────────────

pub fn process_function_signature<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let return_type = extract_ts_return_type(node);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Export,
        metadata: {
            let mut metadata = SymbolMetadata::default();
            metadata.mark_exported();
            metadata.set_return_type(return_type);
            metadata.set_type_parameters(type_params);
            metadata.set_parameters(extract_ts_parameters(node));
            metadata.set_doc_sections(doc_sections);
            metadata
        },
    })
}
