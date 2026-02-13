mod declarations;
mod members;

use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

pub(super) fn process_function_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    declarations::process_function_declaration(node)
}

pub(super) fn process_variable_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<ParsedItem> {
    declarations::process_variable_declaration(node)
}

pub(super) fn process_assignment_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<ParsedItem> {
    members::process_assignment_statement(node)
}

pub(super) fn dedupe(items: Vec<ParsedItem>) -> Vec<ParsedItem> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for item in items {
        let key = format!(
            "{}:{}:{}:{}:{}:{}:{}",
            item.kind,
            item.name,
            item.signature,
            item.metadata.owner_name.as_deref().unwrap_or_default(),
            item.start_line,
            item.end_line,
            item.metadata
                .attributes
                .iter()
                .find(|a| a.starts_with("member_access:"))
                .cloned()
                .unwrap_or_default()
        );
        if seen.insert(key) {
            out.push(item);
        }
    }

    out
}

fn build_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: SymbolKind,
    name: String,
    visibility: Visibility,
    metadata: SymbolMetadata,
    doc_comment: String,
) -> ParsedItem {
    ParsedItem {
        kind,
        name,
        signature: crate::extractors::helpers::extract_signature(node),
        source: crate::extractors::helpers::extract_source(node, 40),
        doc_comment,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    }
}
