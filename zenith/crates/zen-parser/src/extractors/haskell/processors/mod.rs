mod declarations;

use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

pub(super) fn process_module<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    declarations::process_module(node)
}

pub(super) fn process_import<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    declarations::process_import(node)
}

pub(super) fn process_function_like<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    declarations::process_function_like(node)
}

pub(super) fn process_class_decl<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    declarations::process_class_decl(node)
}

pub(super) fn process_type_decl<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    declarations::process_type_decl(node)
}

pub(super) fn dedupe_and_merge(items: Vec<ParsedItem>) -> Vec<ParsedItem> {
    declarations::dedupe_and_merge(items)
}

pub(super) fn build_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: SymbolKind,
    name: String,
    metadata: SymbolMetadata,
) -> ParsedItem {
    ParsedItem {
        kind,
        name,
        signature: crate::extractors::helpers::extract_signature(node),
        source: crate::extractors::helpers::extract_source(node, 40),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Private,
        metadata,
    }
}
