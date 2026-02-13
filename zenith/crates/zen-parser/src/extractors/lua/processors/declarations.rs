use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

use super::super::lua_helpers;
use super::build_item;

pub(super) fn process_function_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    let name_node = node.field("name")?;
    let is_local = node.text().trim_start().starts_with("local function");
    let visibility = lua_helpers::visibility_for_local(is_local);
    let parameters = lua_helpers::extract_parameters(node);
    let doc = lua_helpers::extract_lua_doc_before(node);

    if let Some(member) = lua_helpers::extract_member_name(&name_node) {
        let mut metadata = SymbolMetadata {
            owner_name: Some(member.owner.clone()),
            owner_kind: Some(lua_helpers::owner_kind_for_table()),
            is_static_member: !member.is_method_syntax,
            parameters,
            attributes: vec![format!("member_access:{}", member.access_kind)],
            ..Default::default()
        };
        lua_helpers::apply_luadoc_metadata(&doc, &mut metadata);
        return Some(build_item(
            node,
            SymbolKind::Method,
            member.member,
            visibility,
            metadata,
            doc,
        ));
    }

    let name = name_node.text().to_string();
    let mut metadata = SymbolMetadata {
        parameters,
        ..Default::default()
    };
    lua_helpers::apply_luadoc_metadata(&doc, &mut metadata);
    Some(build_item(
        node,
        SymbolKind::Function,
        name,
        visibility,
        metadata,
        doc,
    ))
}

pub(super) fn process_variable_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<ParsedItem> {
    let node_text = node.text();
    let text = node_text.trim();
    if !text.starts_with("local ") {
        return Vec::new();
    }

    let assignment = node
        .children()
        .find(|c| c.kind().as_ref() == "assignment_statement");
    let values = assignment
        .and_then(|stmt| {
            stmt.children()
                .find(|c| c.kind().as_ref() == "expression_list")
        })
        .map_or_else(Vec::new, |exprs| exprs.children().collect::<Vec<_>>());

    let decl_part = text
        .trim_start_matches("local ")
        .split('=')
        .next()
        .map(str::trim)
        .unwrap_or_default();

    let mut items = Vec::new();
    let doc = lua_helpers::extract_lua_doc_before(node);

    for (idx, token) in decl_part.split(',').map(str::trim).enumerate() {
        if token.is_empty() {
            continue;
        }
        let (name, attrs) = parse_local_binding_token(token);
        if name.is_empty() {
            continue;
        }
        let value = values.get(idx);

        if value.is_some_and(|v| v.kind().as_ref() == "function_definition") {
            let mut metadata = SymbolMetadata {
                parameters: value.map_or_else(Vec::new, lua_helpers::extract_parameters),
                attributes: attrs,
                ..Default::default()
            };
            let local_attrs = metadata.attributes.clone();
            lua_helpers::add_local_attrs(&mut metadata, &local_attrs);
            metadata
                .attributes
                .push("callable_origin:assignment".to_string());
            metadata.attributes.push(format!("callable_alias:{name}"));
            lua_helpers::apply_luadoc_metadata(&doc, &mut metadata);
            items.push(build_item(
                node,
                SymbolKind::Function,
                name,
                lua_helpers::visibility_for_local(true),
                metadata,
                doc.clone(),
            ));
            continue;
        }

        let kind = if attrs.iter().any(|attr| attr == "const") {
            SymbolKind::Const
        } else {
            SymbolKind::Static
        };

        let mut metadata = SymbolMetadata {
            attributes: attrs,
            ..Default::default()
        };
        let local_attrs = metadata.attributes.clone();
        lua_helpers::add_local_attrs(&mut metadata, &local_attrs);
        lua_helpers::apply_luadoc_metadata(&doc, &mut metadata);

        items.push(build_item(
            node,
            kind,
            name.clone(),
            lua_helpers::visibility_for_local(true),
            metadata,
            doc.clone(),
        ));

        if let Some(table_ctor) = value.filter(|v| v.kind().as_ref() == "table_constructor") {
            items.extend(extract_table_constructor_members(
                table_ctor,
                &name,
                &lua_helpers::visibility_for_local(true),
            ));
        }
    }

    items
}

fn parse_local_binding_token(token: &str) -> (String, Vec<String>) {
    let mut attrs = Vec::new();
    let mut name = String::new();

    let mut rest = token.trim();
    if let Some(pos) = rest.find('<') {
        name = rest[..pos].trim().to_string();
    }

    while let Some(start) = rest.find('<') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('>') else {
            break;
        };
        let attr = after_start[..end].trim();
        if !attr.is_empty() {
            attrs.push(attr.to_string());
        }
        rest = &after_start[end + 1..];
    }

    if name.is_empty() {
        name = token.trim().to_string();
    }

    (name, attrs)
}

pub(super) fn extract_table_constructor_members<D: ast_grep_core::Doc>(
    table_constructor: &Node<D>,
    owner_name: &str,
    visibility: &crate::types::Visibility,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();

    for field in table_constructor.children() {
        if field.kind().as_ref() != "field" {
            continue;
        }

        let Some(name_field) = field.field("name") else {
            continue;
        };
        let Some(member_name) = lua_helpers::normalize_member_key(&name_field) else {
            continue;
        };

        let Some(value_field) = field.field("value") else {
            continue;
        };

        let kind = if value_field.kind().as_ref() == "function_definition" {
            SymbolKind::Method
        } else {
            SymbolKind::Field
        };

        let mut metadata = SymbolMetadata {
            owner_name: Some(owner_name.to_string()),
            owner_kind: Some(lua_helpers::owner_kind_for_table()),
            is_static_member: true,
            attributes: vec![format!(
                "member_access:{}",
                if name_field.kind().as_ref() == "string" {
                    "bracket"
                } else {
                    "dot"
                }
            )],
            ..Default::default()
        };

        if kind == SymbolKind::Method {
            metadata.parameters = lua_helpers::extract_parameters(&value_field);
            metadata
                .attributes
                .push("callable_origin:table_ctor".to_string());
        }

        items.push(build_item(
            &field,
            kind,
            member_name,
            visibility.clone(),
            metadata,
            String::new(),
        ));
    }

    items
}
