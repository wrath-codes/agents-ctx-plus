use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

use super::super::cs_helpers;
use super::build_item;

pub(super) fn process_member_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    match node.kind().as_ref() {
        "method_declaration" => single_member(node, SymbolKind::Method),
        "constructor_declaration" => single_member(node, SymbolKind::Constructor),
        "property_declaration" => single_member(node, SymbolKind::Property),
        "indexer_declaration" => single_member(node, SymbolKind::Indexer),
        "event_declaration" => single_member(node, SymbolKind::Event),
        "operator_declaration" | "conversion_operator_declaration" | "destructor_declaration" => {
            single_member(node, SymbolKind::Method)
        }
        "field_declaration" => field_like_members(node, false),
        "event_field_declaration" => field_like_members(node, true),
        _ => Vec::new(),
    }
}

fn single_member<D: ast_grep_core::Doc>(
    node: &Node<D>,
    fallback_kind: SymbolKind,
) -> Vec<ParsedItem> {
    let modifiers = cs_helpers::extract_modifiers(node);
    let visibility = cs_helpers::visibility_from_modifiers(&modifiers);
    let (owner_name, owner_kind) = cs_helpers::owner_from_ancestors(node)
        .unwrap_or_else(|| (String::new(), SymbolKind::Class));

    let kind = if fallback_kind == SymbolKind::Field && cs_helpers::is_const_member(&modifiers) {
        SymbolKind::Const
    } else {
        fallback_kind
    };

    let name =
        member_name(node, kind, &owner_name).unwrap_or_else(|| node.kind().as_ref().to_string());

    let mut metadata = SymbolMetadata {
        owner_name: (!owner_name.is_empty()).then_some(owner_name),
        owner_kind: Some(owner_kind),
        is_static_member: cs_helpers::is_static_member(&modifiers),
        parameters: cs_helpers::extract_parameters(node),
        ..Default::default()
    };

    if matches!(
        kind,
        SymbolKind::Method | SymbolKind::Property | SymbolKind::Indexer | SymbolKind::Event
    ) {
        metadata.return_type = node
            .field("returns")
            .or_else(|| node.field("type"))
            .map(|rtype| rtype.text().to_string());
    }

    vec![build_item(
        node,
        kind,
        name,
        visibility,
        metadata,
        cs_helpers::extract_csharp_doc_before(node),
    )]
}

fn field_like_members<D: ast_grep_core::Doc>(node: &Node<D>, is_event: bool) -> Vec<ParsedItem> {
    let modifiers = cs_helpers::extract_modifiers(node);
    let visibility = cs_helpers::visibility_from_modifiers(&modifiers);
    let (owner_name, owner_kind) = cs_helpers::owner_from_ancestors(node)
        .unwrap_or_else(|| (String::new(), SymbolKind::Class));
    let names = cs_helpers::extract_variable_names_from_declaration(node);

    let kind = if is_event {
        SymbolKind::Event
    } else if cs_helpers::is_const_member(&modifiers) {
        SymbolKind::Const
    } else {
        SymbolKind::Field
    };

    names
        .into_iter()
        .map(|name| {
            let metadata = SymbolMetadata {
                owner_name: (!owner_name.is_empty()).then_some(owner_name.clone()),
                owner_kind: Some(owner_kind),
                is_static_member: cs_helpers::is_static_member(&modifiers),
                return_type: node
                    .children()
                    .find(|child| child.kind().as_ref() == "variable_declaration")
                    .and_then(|decl| decl.field("type"))
                    .map(|rtype| rtype.text().to_string()),
                ..Default::default()
            };
            build_item(
                node,
                kind,
                name,
                visibility.clone(),
                metadata,
                cs_helpers::extract_csharp_doc_before(node),
            )
        })
        .collect()
}

fn member_name<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: SymbolKind,
    owner_name: &str,
) -> Option<String> {
    if kind == SymbolKind::Indexer {
        return Some("this[]".to_string());
    }
    if node.kind().as_ref() == "operator_declaration" {
        if let Some(op) = node.field("operator") {
            return Some(format!("operator{}", op.text()));
        }
        return Some("operator".to_string());
    }
    if node.kind().as_ref() == "conversion_operator_declaration" {
        return node.field("type").map(|t| format!("operator {}", t.text()));
    }
    if node.kind().as_ref() == "destructor_declaration" {
        if owner_name.is_empty() {
            return Some("destructor".to_string());
        }
        return Some(format!("~{owner_name}"));
    }
    node.field("name").map(|n| n.text().to_string())
}
