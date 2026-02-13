use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata};

use super::super::php_helpers;
use super::build_item;
use super::types;

pub(super) fn process_member_like<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    match node.kind().as_ref() {
        "method_declaration" => process_method(node),
        "property_declaration" => process_property_declaration(node),
        "const_declaration" => process_const_declaration(node),
        "property_promotion_parameter" => process_promoted_property(node),
        "enum_case" => process_enum_case(node),
        "use_declaration" => process_trait_use_declaration(node),
        "global_declaration" => process_global_declaration(node),
        "function_static_declaration" => process_function_static_declaration(node),
        _ => Vec::new(),
    }
}

fn process_method<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let Some(name_node) = node.field("name") else {
        return Vec::new();
    };
    let name = name_node.text().to_string();
    let doc = php_helpers::extract_doc_before(node);
    let owner = php_helpers::owner_from_ancestors(node);

    let kind = if name == "__construct" {
        SymbolKind::Constructor
    } else {
        SymbolKind::Method
    };

    let mut metadata = SymbolMetadata {
        owner_name: owner.as_ref().map(|(n, _)| n.clone()),
        owner_kind: owner.as_ref().map(|(_, k)| *k),
        is_static_member: php_helpers::is_static(node),
        parameters: php_helpers::extract_parameter_descriptors(node),
        return_type: types::normalize_type_node(node.field("return_type")),
        attributes: php_helpers::extract_attributes(node),
        ..Default::default()
    };
    php_helpers::apply_phpdoc_metadata(&doc, &mut metadata);

    vec![build_item(
        node,
        kind,
        name,
        php_helpers::extract_visibility(node),
        metadata,
        doc,
    )]
}

fn process_property_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let owner = php_helpers::owner_from_ancestors(node);
    let visibility = php_helpers::extract_visibility(node);
    let is_static = php_helpers::is_static(node);
    let attrs = php_helpers::extract_attributes(node);
    let ty = types::normalize_type_node(node.field("type"));

    let mut items: Vec<ParsedItem> = node
        .children()
        .filter(|c| c.kind().as_ref() == "property_element")
        .filter_map(|property| {
            let name = property
                .field("name")
                .map(|n| n.text().to_string())?
                .trim_start_matches('$')
                .to_string();

            let metadata = SymbolMetadata {
                owner_name: owner.as_ref().map(|(n, _)| n.clone()),
                owner_kind: owner.as_ref().map(|(_, k)| *k),
                is_static_member: is_static,
                return_type: ty.clone(),
                attributes: attrs.clone(),
                ..Default::default()
            };

            Some(build_item(
                &property,
                SymbolKind::Property,
                name,
                visibility.clone(),
                metadata,
                String::new(),
            ))
        })
        .collect();

    for hook_list in node
        .children()
        .filter(|c| c.kind().as_ref() == "property_hook_list")
    {
        for hook in hook_list
            .children()
            .filter(|c| c.kind().as_ref() == "property_hook")
        {
            if let Some(hook_name) = hook
                .children()
                .find(|c| c.kind().as_ref() == "name")
                .map(|n| n.text().to_string())
            {
                let hook_params = php_helpers::extract_parameter_descriptors(&hook);
                let hook_return = types::normalize_type_node(hook.field("type"))
                    .or_else(|| types::normalize_type_node(hook.field("return_type")))
                    .unwrap_or_default();
                let hook_attrs = php_helpers::extract_attributes(&hook);

                let mut hook_desc = format!("hook:{hook_name}");
                if !hook_params.is_empty() {
                    hook_desc.push_str(":params(");
                    hook_desc.push_str(&hook_params.join(", "));
                    hook_desc.push(')');
                }
                if !hook_return.is_empty() {
                    hook_desc.push_str(":return(");
                    hook_desc.push_str(&hook_return);
                    hook_desc.push(')');
                }

                for item in &mut items {
                    item.metadata.methods.push(hook_desc.clone());
                    item.metadata
                        .attributes
                        .push(format!("property_hook:name:{hook_name}"));
                    if !hook_return.is_empty() {
                        item.metadata
                            .attributes
                            .push(format!("property_hook:return:{hook_return}"));
                    }
                    item.metadata.attributes.extend(
                        hook_params
                            .iter()
                            .map(|param| format!("property_hook:param:{param}")),
                    );
                    item.metadata.attributes.extend(
                        hook_attrs
                            .iter()
                            .map(|attr| format!("property_hook:attr:{attr}")),
                    );
                }
            }
        }
    }

    items
}

fn process_const_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let owner = php_helpers::owner_from_ancestors(node);
    let visibility = if owner.is_some() {
        php_helpers::extract_visibility(node)
    } else {
        crate::types::Visibility::Public
    };
    let ty = types::normalize_type_node(node.field("type"));
    let attrs = php_helpers::extract_attributes(node);

    node.children()
        .filter(|c| c.kind().as_ref() == "const_element")
        .filter_map(|elem| {
            let name = elem
                .children()
                .find(|c| c.kind().as_ref() == "name")
                .map(|n| n.text().to_string())?;

            let metadata = SymbolMetadata {
                owner_name: owner.as_ref().map(|(n, _)| n.clone()),
                owner_kind: owner.as_ref().map(|(_, k)| *k),
                is_static_member: owner.is_some(),
                return_type: ty.clone(),
                attributes: attrs.clone(),
                ..Default::default()
            };

            Some(build_item(
                &elem,
                SymbolKind::Const,
                name,
                visibility.clone(),
                metadata,
                String::new(),
            ))
        })
        .collect()
}

fn process_promoted_property<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let owner = php_helpers::owner_from_ancestors(node);
    let Some((owner_name, owner_kind)) = owner else {
        return Vec::new();
    };

    let name = node
        .field("name")
        .map(|n| n.text().to_string())
        .unwrap_or_default()
        .trim_start_matches('$')
        .to_string();

    if name.is_empty() {
        return Vec::new();
    }

    let metadata = SymbolMetadata {
        owner_name: Some(owner_name),
        owner_kind: Some(owner_kind),
        is_static_member: false,
        return_type: types::normalize_type_node(node.field("type")),
        attributes: php_helpers::extract_attributes(node),
        ..Default::default()
    };

    vec![build_item(
        node,
        SymbolKind::Field,
        name,
        php_helpers::extract_visibility(node),
        metadata,
        String::new(),
    )]
}

fn process_trait_use_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let owner = php_helpers::owner_from_ancestors(node);
    let Some((owner_name, owner_kind)) = owner else {
        return Vec::new();
    };

    let mut items = Vec::new();

    items.extend(trait_use_targets(
        node,
        &owner_name,
        owner_kind,
        &php_helpers::extract_visibility(node),
    ));

    for use_list in node
        .children()
        .filter(|child| child.kind().as_ref() == "use_list")
    {
        for clause in use_list.children() {
            let text = clause.text().to_string();
            if text.is_empty() {
                continue;
            }

            let attrs = trait_adaptation_attrs(&clause, &text);

            let metadata = SymbolMetadata {
                owner_name: Some(owner_name.clone()),
                owner_kind: Some(owner_kind),
                attributes: attrs,
                ..Default::default()
            };

            items.push(build_item(
                &clause,
                SymbolKind::Module,
                text,
                php_helpers::extract_visibility(node),
                metadata,
                String::new(),
            ));
        }
    }

    items
}

fn trait_use_targets<D: ast_grep_core::Doc>(
    node: &Node<D>,
    owner_name: &str,
    owner_kind: SymbolKind,
    visibility: &crate::types::Visibility,
) -> Vec<ParsedItem> {
    node.children()
        .filter(|child| {
            matches!(
                child.kind().as_ref(),
                "name" | "qualified_name" | "relative_name"
            )
        })
        .map(|trait_name| {
            let metadata = SymbolMetadata {
                owner_name: Some(owner_name.to_string()),
                owner_kind: Some(owner_kind),
                attributes: vec!["trait_use".to_string()],
                ..Default::default()
            };

            build_item(
                &trait_name,
                SymbolKind::Module,
                trait_name.text().to_string(),
                visibility.clone(),
                metadata,
                String::new(),
            )
        })
        .collect()
}

fn trait_adaptation_attrs<D: ast_grep_core::Doc>(clause: &Node<D>, text: &str) -> Vec<String> {
    let kind = clause.kind();
    let names = clause
        .children()
        .filter(|c| {
            matches!(
                c.kind().as_ref(),
                "name" | "class_constant_access_expression"
            )
        })
        .map(|c| c.text().to_string())
        .collect::<Vec<_>>();

    if kind.as_ref() == "use_as_clause" {
        let mut attrs = vec!["trait_use:mode=as".to_string()];
        if let Some(first) = names.first() {
            attrs.push(format!("trait_use:target={first}"));
        }
        if names.len() > 1 {
            attrs.push(format!("trait_use:alias={}", names[1]));
        }
        if let Some(vis) = clause
            .children()
            .find(|c| c.kind().as_ref() == "visibility_modifier")
            .map(|v| v.text().to_string())
        {
            attrs.push(format!("trait_use:visibility={vis}"));
        }
        return attrs;
    }

    let mut attrs = vec!["trait_use:mode=insteadof".to_string()];
    if let Some(first) = names.first() {
        attrs.push(format!("trait_use:target={first}"));
    }
    for n in names.iter().skip(1) {
        attrs.push(format!("trait_use:instead_of={n}"));
    }

    if let Some((_, rhs)) = text.split_once("insteadof") {
        for name in rhs.split(',').map(str::trim) {
            let clean = name.trim_end_matches(';').trim();
            if !clean.is_empty() {
                attrs.push(format!("trait_use:instead_of={clean}"));
            }
        }
    }

    attrs
}

fn process_global_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let owner = php_helpers::owner_callable_from_ancestors(node);

    node.children()
        .filter(|child| {
            matches!(
                child.kind().as_ref(),
                "variable_name" | "dynamic_variable_name"
            )
        })
        .map(|var| {
            let name = var
                .text()
                .trim_start_matches('$')
                .trim_start_matches('{')
                .trim_end_matches('}')
                .to_string();

            let metadata = SymbolMetadata {
                owner_name: owner.as_ref().map(|(n, _)| n.clone()),
                owner_kind: owner.as_ref().map(|(_, k)| *k),
                attributes: vec!["global_declaration".to_string()],
                ..Default::default()
            };

            build_item(
                &var,
                SymbolKind::Static,
                name,
                crate::types::Visibility::Private,
                metadata,
                String::new(),
            )
        })
        .collect()
}

fn process_function_static_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let owner = php_helpers::owner_callable_from_ancestors(node);

    node.children()
        .filter(|child| child.kind().as_ref() == "static_variable_declaration")
        .filter_map(|decl| {
            let name = decl
                .field("name")
                .map(|n| n.text().to_string())?
                .trim_start_matches('$')
                .to_string();

            let metadata = SymbolMetadata {
                owner_name: owner.as_ref().map(|(n, _)| n.clone()),
                owner_kind: owner.as_ref().map(|(_, k)| *k),
                attributes: vec!["function_static".to_string()],
                ..Default::default()
            };

            Some(build_item(
                &decl,
                SymbolKind::Static,
                name,
                crate::types::Visibility::Private,
                metadata,
                String::new(),
            ))
        })
        .collect()
}

fn process_enum_case<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let owner = php_helpers::owner_from_ancestors(node);
    let Some((owner_name, owner_kind)) = owner else {
        return Vec::new();
    };

    let Some(name) = node.field("name").map(|n| n.text().to_string()) else {
        return Vec::new();
    };

    let metadata = SymbolMetadata {
        owner_name: Some(owner_name),
        owner_kind: Some(owner_kind),
        is_static_member: true,
        ..Default::default()
    };

    vec![build_item(
        node,
        SymbolKind::Const,
        name,
        crate::types::Visibility::Public,
        metadata,
        String::new(),
    )]
}
