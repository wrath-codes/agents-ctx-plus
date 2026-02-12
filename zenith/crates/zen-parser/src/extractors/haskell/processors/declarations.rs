use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

use super::super::hs_helpers;
use super::build_item;

pub(super) fn process_module<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = hs_helpers::extract_module_name(node)?;
    Some(build_item(
        node,
        SymbolKind::Module,
        name,
        SymbolMetadata::default(),
    ))
}

pub(super) fn process_import<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = hs_helpers::extract_module_name(node)?;
    Some(build_item(
        node,
        SymbolKind::Module,
        name,
        SymbolMetadata::default(),
    ))
}

pub(super) fn process_function_like<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = hs_helpers::extract_name(node)?;
    Some(build_item(
        node,
        SymbolKind::Function,
        name,
        SymbolMetadata::default(),
    ))
}

pub(super) fn process_class_decl<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = hs_helpers::extract_name(node)?;
    Some(build_item(
        node,
        SymbolKind::Trait,
        name,
        SymbolMetadata::default(),
    ))
}

pub(super) fn process_type_decl<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = hs_helpers::extract_name(node)?;
    let mut metadata = SymbolMetadata::default();

    let kind = match node.kind().as_ref() {
        "data_type" => {
            let kind = hs_helpers::classify_data_type_hybrid(node);
            if kind == SymbolKind::Enum {
                metadata.variants = hs_helpers::extract_data_constructors(node);
            }
            if kind == SymbolKind::Struct {
                metadata.fields = hs_helpers::extract_record_fields(node);
            }
            kind
        }
        "newtype" | "type_family" | "type_instance" => SymbolKind::TypeAlias,
        _ => return None,
    };

    Some(build_item(node, kind, name, metadata))
}

pub(super) fn dedupe_and_merge(items: Vec<ParsedItem>) -> Vec<ParsedItem> {
    let mut deduped: Vec<ParsedItem> = Vec::new();

    for item in items {
        if let Some(existing) = deduped
            .iter_mut()
            .find(|existing| existing.kind == item.kind && existing.name == item.name)
        {
            if existing.signature.contains("::") && !item.signature.contains("::") {
                if item.source.is_some() {
                    existing.source.clone_from(&item.source);
                }
                existing.start_line = existing.start_line.min(item.start_line);
                existing.end_line = existing.end_line.max(item.end_line);
            } else if !existing.signature.contains("::") && item.signature.contains("::") {
                existing.signature = item.signature;
            }

            if existing.metadata.variants.is_empty() && !item.metadata.variants.is_empty() {
                existing.metadata.variants = item.metadata.variants;
            }
            if existing.metadata.fields.is_empty() && !item.metadata.fields.is_empty() {
                existing.metadata.fields = item.metadata.fields;
            }
        } else {
            deduped.push(item);
        }
    }

    deduped
}
