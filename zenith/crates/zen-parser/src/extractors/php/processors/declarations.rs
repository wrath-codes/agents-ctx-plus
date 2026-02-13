use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::php_helpers;
use super::build_item;
use super::types;

pub(super) fn process_module_like<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let name = match node.kind().as_ref() {
        "namespace_definition" => node
            .field("name")
            .map_or_else(|| "global".to_string(), |n| n.text().to_string()),
        _ => return None,
    };

    if name.is_empty() {
        return None;
    }

    Some(build_item(
        node,
        SymbolKind::Module,
        name,
        Visibility::Public,
        SymbolMetadata::default(),
        php_helpers::extract_doc_before(node),
    ))
}

pub(super) fn process_function_definition<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    let name = node.field("name")?.text().to_string();
    let doc = php_helpers::extract_doc_before(node);

    let mut metadata = SymbolMetadata {
        parameters: php_helpers::extract_parameter_descriptors(node),
        return_type: types::normalize_type_node(node.field("return_type")),
        attributes: php_helpers::extract_attributes(node),
        ..Default::default()
    };
    php_helpers::apply_phpdoc_metadata(&doc, &mut metadata);

    Some(build_item(
        node,
        SymbolKind::Function,
        name,
        Visibility::Public,
        metadata,
        doc,
    ))
}

pub(super) fn process_type_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    let kind = match node.kind().as_ref() {
        "class_declaration" => SymbolKind::Class,
        "interface_declaration" => SymbolKind::Interface,
        "trait_declaration" => SymbolKind::Trait,
        "enum_declaration" => SymbolKind::Enum,
        _ => return None,
    };

    let name = node.field("name")?.text().to_string();
    let doc = php_helpers::extract_doc_before(node);
    let mut metadata = SymbolMetadata {
        base_classes: php_helpers::extract_type_bases(node),
        attributes: php_helpers::extract_attributes(node),
        ..Default::default()
    };
    php_helpers::apply_phpdoc_metadata(&doc, &mut metadata);
    if kind == SymbolKind::Enum {
        metadata.variants = php_helpers::collect_enum_variants(node);
    }

    Some(build_item(
        node,
        kind,
        name,
        Visibility::Public,
        metadata,
        doc,
    ))
}
