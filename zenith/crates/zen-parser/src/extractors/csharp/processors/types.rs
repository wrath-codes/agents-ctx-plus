use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

use super::super::cs_helpers;
use super::build_item;

pub(super) fn process_type_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    let kind = match node.kind().as_ref() {
        "class_declaration" | "record_declaration" => SymbolKind::Class,
        "struct_declaration" => SymbolKind::Struct,
        "interface_declaration" => SymbolKind::Interface,
        "enum_declaration" => SymbolKind::Enum,
        "delegate_declaration" => SymbolKind::TypeAlias,
        _ => return None,
    };

    let name = node.field("name").map(|n| n.text().to_string())?;
    let modifiers = cs_helpers::extract_modifiers(node);
    let visibility = cs_helpers::visibility_from_modifiers(&modifiers);
    let mut metadata = SymbolMetadata {
        type_parameters: node
            .field("type_parameters")
            .map(|tp| tp.text().to_string()),
        base_classes: cs_helpers::extract_base_types(node),
        ..Default::default()
    };

    if node.kind().as_ref() == "enum_declaration" {
        metadata.variants = extract_enum_variants(node);
    }
    if node.kind().as_ref() == "delegate_declaration" {
        metadata.return_type = node.field("type").map(|t| t.text().to_string());
        metadata.parameters = cs_helpers::extract_parameters(node);
    }

    Some(build_item(
        node,
        kind,
        name,
        visibility,
        metadata,
        cs_helpers::extract_csharp_doc_before(node),
    ))
}

fn extract_enum_variants<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .find(|child| child.kind().as_ref() == "enum_member_declaration_list")
        .map(|members| {
            members
                .children()
                .filter(|child| child.kind().as_ref() == "enum_member_declaration")
                .filter_map(|variant| variant.field("name").map(|name| name.text().to_string()))
                .collect()
        })
        .unwrap_or_default()
}
