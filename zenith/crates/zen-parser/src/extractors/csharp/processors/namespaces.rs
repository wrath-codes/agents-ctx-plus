use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::cs_helpers;
use super::build_item;

pub(super) fn process_using_directive<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let signature = crate::extractors::helpers::extract_signature(node);
    let name = signature
        .trim_start_matches("using")
        .trim()
        .trim_end_matches(';')
        .to_string();
    if name.is_empty() {
        return None;
    }

    Some(build_item(
        node,
        SymbolKind::Module,
        name,
        Visibility::Public,
        SymbolMetadata::default(),
        String::new(),
    ))
}

pub(super) fn process_namespace<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    Some(build_item(
        node,
        SymbolKind::Module,
        name,
        Visibility::Public,
        SymbolMetadata::default(),
        cs_helpers::extract_csharp_doc_before(node),
    ))
}
