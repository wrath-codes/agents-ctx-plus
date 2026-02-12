use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, TypeScriptMetadataExt, Visibility};

use super::super::ts_helpers::{
    extract_jsdoc_before, extract_ts_parameters, extract_ts_return_type, parse_jsdoc_sections,
};
use super::classes::process_class;
use super::functions::process_function_signature;

// ── ambient_declaration (declare ...) ──────────────────────────────

pub fn process_ambient_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    for child in node.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_signature" => {
                if let Some(item) = process_function_signature(&child, node) {
                    items.push(item);
                }
            }
            "lexical_declaration" => {
                items.extend(process_lexical_declaration(&child, node, false));
            }
            "class_declaration" => {
                if let Some(item) = process_class(&child, node, false, false) {
                    items.push(item);
                }
            }
            "module" => {
                if let Some(name_node) = child.field("name") {
                    let name = name_node.text().to_string();
                    items.push(ParsedItem {
                        kind: SymbolKind::Module,
                        name: name.trim_matches('"').to_string(),
                        signature: format!("declare module {name}"),
                        source: helpers::extract_source(&child, 50),
                        doc_comment: String::new(),
                        start_line: node.start_pos().line() as u32 + 1,
                        end_line: node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Export,
                        metadata: SymbolMetadata::default(),
                    });
                }
            }
            _ => {}
        }
    }
    items
}

// ── variable_declaration (var/let) ─────────────────────────────────

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

// ── lexical_declaration (arrow functions, consts) ──────────────────

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
        let return_type = extract_ts_return_type(&arrow);
        let params = extract_ts_parameters(&arrow);
        let type_params = arrow
            .field("type_parameters")
            .map(|tp| tp.text().to_string());

        Some(ParsedItem {
            kind: SymbolKind::Function,
            name,
            signature: helpers::extract_signature(declaration),
            source: helpers::extract_source(declaration, 50),
            doc_comment: jsdoc,
            start_line: declaration.start_pos().line() as u32 + 1,
            end_line: declaration.end_pos().line() as u32 + 1,
            visibility,
            metadata: {
                let mut metadata = SymbolMetadata::default();
                if is_async {
                    metadata.mark_async();
                }
                if is_exported {
                    metadata.mark_exported();
                }
                metadata.set_return_type(return_type);
                metadata.set_type_parameters(type_params);
                metadata.set_parameters(params);
                metadata.set_doc_sections(doc_sections);
                metadata
            },
        })
    } else {
        let type_annotation = declarator
            .children()
            .find(|c| c.kind().as_ref() == "type_annotation")
            .map(|ta| {
                ta.text()
                    .to_string()
                    .trim_start_matches(':')
                    .trim()
                    .to_string()
            });

        Some(ParsedItem {
            kind: SymbolKind::Const,
            name,
            signature: helpers::extract_signature(declaration),
            source: helpers::extract_source(declaration, 50),
            doc_comment: jsdoc,
            start_line: declaration.start_pos().line() as u32 + 1,
            end_line: declaration.end_pos().line() as u32 + 1,
            visibility,
            metadata: {
                let mut metadata = SymbolMetadata::default();
                if is_exported {
                    metadata.mark_exported();
                }
                metadata.set_return_type(type_annotation);
                metadata.set_doc_sections(doc_sections);
                metadata
            },
        })
    }
}
