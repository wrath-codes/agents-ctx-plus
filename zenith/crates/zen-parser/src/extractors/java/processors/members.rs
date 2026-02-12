use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

use super::super::java_helpers;
use super::build_item;

pub(super) fn process_member_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    match node.kind().as_ref() {
        "method_declaration" | "annotation_type_element_declaration" => {
            single_member(node, SymbolKind::Method)
        }
        "constructor_declaration" | "compact_constructor_declaration" => {
            single_member(node, SymbolKind::Constructor)
        }
        "field_declaration" => field_like_members(node, false),
        "constant_declaration" => field_like_members(node, true),
        _ => Vec::new(),
    }
}

fn single_member<D: ast_grep_core::Doc>(
    node: &Node<D>,
    fallback_kind: SymbolKind,
) -> Vec<ParsedItem> {
    let modifiers = java_helpers::extract_modifiers(node);
    let visibility = java_helpers::visibility_from_modifiers(&modifiers);
    let owner = java_helpers::owner_from_ancestors(node);

    let kind = if fallback_kind == SymbolKind::Method && owner.is_none() {
        SymbolKind::Function
    } else {
        fallback_kind
    };

    let name = node.field("name").map_or_else(
        || node.kind().as_ref().to_string(),
        |name| name.text().to_string(),
    );

    let mut metadata = SymbolMetadata {
        owner_name: owner.as_ref().map(|(owner_name, _)| owner_name.clone()),
        owner_kind: owner.as_ref().map(|(_, owner_kind)| *owner_kind),
        is_static_member: java_helpers::is_static_member(&modifiers),
        parameters: java_helpers::extract_parameters(node),
        type_parameters: node
            .field("type_parameters")
            .map(|type_params| type_params.text().to_string()),
        attributes: java_helpers::extract_annotations(node),
        ..Default::default()
    };

    if let Some(throws) = java_helpers::extract_throws(node) {
        metadata.attributes.push(throws);
    }

    if node.kind().as_ref() == "annotation_type_element_declaration"
        && let Some(default_value) = node.field("value")
    {
        metadata
            .attributes
            .push(format!("default {}", default_value.text()));
    }

    if matches!(kind, SymbolKind::Method | SymbolKind::Function) {
        metadata.return_type = node
            .field("type")
            .map(|return_type| return_type.text().to_string());
    }

    vec![build_item(
        node,
        kind,
        name,
        visibility,
        metadata,
        java_helpers::extract_javadoc_before(node),
    )]
}

fn field_like_members<D: ast_grep_core::Doc>(node: &Node<D>, force_const: bool) -> Vec<ParsedItem> {
    let modifiers = java_helpers::extract_modifiers(node);
    let visibility = java_helpers::visibility_from_modifiers(&modifiers);
    let owner = java_helpers::owner_from_ancestors(node);
    let names = java_helpers::extract_variable_names_from_declaration(node);

    let kind = if force_const || java_helpers::is_const_member(&modifiers) {
        SymbolKind::Const
    } else {
        SymbolKind::Field
    };

    names
        .into_iter()
        .map(|name| {
            let metadata = SymbolMetadata {
                owner_name: owner.as_ref().map(|(owner_name, _)| owner_name.clone()),
                owner_kind: owner.as_ref().map(|(_, owner_kind)| *owner_kind),
                is_static_member: java_helpers::is_static_member(&modifiers),
                return_type: node
                    .field("type")
                    .map(|field_type| field_type.text().to_string()),
                attributes: java_helpers::extract_annotations(node),
                ..Default::default()
            };

            build_item(
                node,
                kind,
                name,
                visibility.clone(),
                metadata,
                java_helpers::extract_javadoc_before(node),
            )
        })
        .collect()
}
