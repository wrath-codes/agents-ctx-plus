use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, SymbolMetadata};

use super::declarations;
use super::functions;
use super::impl_blocks;
use super::types;

/// Process one matched top-level-ish Rust item node.
pub(in super::super) fn process_match_node<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Vec<ParsedItem> {
    let kind = node.kind();
    match kind.as_ref() {
        "impl_item" => impl_blocks::process_impl_item(node, source),
        // Skip function_items that live inside impl/trait bodies —
        // they are already handled by process_impl_item / build_trait_metadata.
        "function_item" => {
            if !is_nested_in_body(node)
                && let Some(item) = process_rust_node(node, source)
            {
                vec![item]
            } else {
                Vec::new()
            }
        }
        "foreign_mod_item" => declarations::process_foreign_mod(node, source),
        "use_declaration" => declarations::process_use_declaration(node, source)
            .into_iter()
            .collect(),
        "extern_crate_declaration" => declarations::process_extern_crate(node, source)
            .into_iter()
            .collect(),
        "macro_invocation" => declarations::process_macro_invocation(node, source)
            .into_iter()
            .collect(),
        _ => process_rust_node(node, source).into_iter().collect(),
    }
}

pub(super) fn process_rust_node<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> Option<ParsedItem> {
    let kind_str = node.kind();
    let k = kind_str.as_ref();

    let name = extract_name(node)?;
    let (symbol_kind, metadata) = build_metadata(node, k, source, &name);

    Some(ParsedItem {
        kind: symbol_kind,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: helpers::extract_doc_comments_rust(node, source),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: helpers::extract_visibility_rust(node),
        metadata,
    })
}

pub(super) fn extract_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("name")
        .map(|n| n.text().to_string())
        .or_else(|| {
            node.children()
                .find(|c| {
                    let k = c.kind();
                    k.as_ref() == "identifier" || k.as_ref() == "type_identifier"
                })
                .map(|c| c.text().to_string())
        })
        .filter(|n| !n.is_empty())
}

/// Check if a node is nested inside a `declaration_list` (impl/trait body).
/// This prevents double-extraction of methods as free functions.
fn is_nested_in_body<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        let k = parent.kind();
        if k.as_ref() == "declaration_list" {
            return true;
        }
        // Stop at source_file level
        if k.as_ref() == "source_file" {
            return false;
        }
        current = parent.parent();
    }
    false
}

fn build_metadata<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: &str,
    source: &str,
    name: &str,
) -> (crate::types::SymbolKind, SymbolMetadata) {
    match kind {
        "function_item" => functions::build_function_metadata(node, source, name),
        "struct_item" | "union_item" => types::build_struct_metadata(node, source, name),
        "enum_item" => types::build_enum_metadata(node, source, name),
        "trait_item" => types::build_trait_metadata(node, source),
        "type_item" => types::build_type_alias_metadata(node),
        "const_item" => types::build_const_metadata(node),
        "static_item" => types::build_static_metadata(node),
        "macro_definition" => functions::build_macro_metadata(node, source),
        "mod_item" => (crate::types::SymbolKind::Module, SymbolMetadata::default()),
        _ => (
            crate::types::SymbolKind::Function,
            SymbolMetadata::default(),
        ),
    }
}
