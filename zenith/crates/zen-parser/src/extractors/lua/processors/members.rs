use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

use super::super::lua_helpers;
use super::build_item;
use super::declarations;

pub(super) fn process_assignment_statement<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<ParsedItem> {
    let Some(variable_list) = node
        .children()
        .find(|c| c.kind().as_ref() == "variable_list")
    else {
        return Vec::new();
    };
    let Some(expression_list) = node
        .children()
        .find(|c| c.kind().as_ref() == "expression_list")
    else {
        return Vec::new();
    };

    let values = expression_list.children().collect::<Vec<_>>();
    let doc = lua_helpers::extract_lua_doc_before(node);
    let mut items = Vec::new();

    for (idx, variable) in variable_list.children().enumerate() {
        let value = values.get(idx);

        if variable.kind().as_ref() == "identifier" {
            let name = variable.text().to_string();
            let kind = if value.is_some_and(|v| v.kind().as_ref() == "function_definition") {
                SymbolKind::Function
            } else {
                SymbolKind::Static
            };

            let mut metadata = SymbolMetadata {
                parameters: value.map_or_else(Vec::new, lua_helpers::extract_parameters),
                ..Default::default()
            };
            if kind == SymbolKind::Function {
                metadata
                    .attributes
                    .push("callable_origin:assignment".to_string());
                metadata.attributes.push(format!("callable_alias:{name}"));
            }
            lua_helpers::apply_luadoc_metadata(&doc, &mut metadata);

            items.push(build_item(
                node,
                kind,
                name.clone(),
                lua_helpers::visibility_for_local(false),
                metadata,
                doc.clone(),
            ));

            if let Some(table_ctor) = value.filter(|v| v.kind().as_ref() == "table_constructor") {
                items.extend(declarations::extract_table_constructor_members(
                    table_ctor,
                    &name,
                    &lua_helpers::visibility_for_local(false),
                ));
            }

            continue;
        }

        if let Some(member) = lua_helpers::extract_member_name(&variable) {
            let kind = if value.is_some_and(|v| v.kind().as_ref() == "function_definition") {
                SymbolKind::Method
            } else {
                SymbolKind::Field
            };

            let mut metadata = SymbolMetadata {
                owner_name: Some(member.owner),
                owner_kind: Some(lua_helpers::owner_kind_for_table()),
                is_static_member: true,
                attributes: vec![format!("member_access:{}", member.access_kind)],
                ..Default::default()
            };

            if let Some(function_value) =
                value.filter(|v| v.kind().as_ref() == "function_definition")
            {
                metadata.parameters = lua_helpers::extract_parameters(function_value);
                metadata
                    .attributes
                    .push("callable_origin:table_field".to_string());
            }
            lua_helpers::apply_luadoc_metadata(&doc, &mut metadata);

            items.push(build_item(
                node,
                kind,
                member.member,
                lua_helpers::visibility_for_local(false),
                metadata,
                doc.clone(),
            ));
        }
    }

    items
}
