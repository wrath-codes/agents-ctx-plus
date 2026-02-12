use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{JavaScriptMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::js_helpers::{extract_js_parameters, extract_jsdoc_before, parse_jsdoc_sections};

// ── lexical_declaration (const/let with arrow functions or values) ─

pub fn process_lexical_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "variable_declarator"
            && let Some(item) = process_variable_declarator(&child, node, jsdoc_anchor, is_exported)
        {
            items.push(item);
        }
    }
    items
}

// ── variable_declaration (var) ─────────────────────────────────────

pub fn process_variable_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "variable_declarator"
            && let Some(item) = process_variable_declarator(&child, node, jsdoc_anchor, is_exported)
        {
            items.push(item);
        }
    }
    items
}

fn process_variable_declarator<D: ast_grep_core::Doc>(
    declarator: &Node<D>,
    declaration: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = declarator.field("name").map(|n| n.text().to_string())?;

    let value = declarator.field("value");
    let is_arrow = value
        .as_ref()
        .is_some_and(|v| v.kind().as_ref() == "arrow_function");

    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    if is_arrow {
        let arrow = value.unwrap();
        let is_async = arrow.children().any(|c| c.kind().as_ref() == "async");
        let params = extract_js_parameters(&arrow);

        let mut metadata = SymbolMetadata::default();
        if is_async {
            metadata.mark_async();
        }
        if is_exported {
            metadata.mark_exported();
        }
        metadata.set_parameters(params);
        metadata.set_doc_sections(doc_sections);

        Some(ParsedItem {
            kind: SymbolKind::Function,
            name,
            signature: helpers::extract_signature(declaration),
            source: helpers::extract_source(declaration, 50),
            doc_comment: jsdoc,
            start_line: declaration.start_pos().line() as u32 + 1,
            end_line: declaration.end_pos().line() as u32 + 1,
            visibility,
            metadata,
        })
    } else {
        let value_kind = declaration_value_kind(declaration);
        let mut metadata = SymbolMetadata::default();
        if is_exported {
            metadata.mark_exported();
        }
        metadata.set_doc_sections(doc_sections);

        Some(ParsedItem {
            kind: value_kind,
            name,
            signature: helpers::extract_signature(declaration),
            source: helpers::extract_source(declaration, 50),
            doc_comment: jsdoc,
            start_line: declaration.start_pos().line() as u32 + 1,
            end_line: declaration.end_pos().line() as u32 + 1,
            visibility,
            metadata,
        })
    }
}

fn declaration_value_kind<D: ast_grep_core::Doc>(declaration: &Node<D>) -> SymbolKind {
    for child in declaration.children() {
        match child.kind().as_ref() {
            "const" => return SymbolKind::Const,
            "let" | "var" => return SymbolKind::Static,
            _ => {}
        }
    }
    SymbolKind::Static
}
