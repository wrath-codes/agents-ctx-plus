mod members;
mod namespaces;
mod types;

use ast_grep_core::Node;

use crate::types::ParsedItem;

pub(super) fn process_member_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    members::process_member_declaration(node)
}

pub(super) fn process_namespace<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    namespaces::process_namespace(node)
}

pub(super) fn process_using_directive<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    namespaces::process_using_directive(node)
}

pub(super) fn process_type_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    types::process_type_declaration(node)
}

pub(super) fn build_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: crate::types::SymbolKind,
    name: String,
    visibility: crate::types::Visibility,
    metadata: crate::types::SymbolMetadata,
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
