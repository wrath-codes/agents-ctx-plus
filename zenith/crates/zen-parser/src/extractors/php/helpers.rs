use ast_grep_core::Node;

use crate::types::{SymbolKind, SymbolMetadata, Visibility};

use super::processors::{phpdoc, types};

pub(super) fn extract_doc_before<D: ast_grep_core::Doc>(anchor: &Node<D>) -> String {
    let mut docs = Vec::new();
    let mut current = anchor.prev();

    while let Some(prev) = current {
        if prev.kind().as_ref() != "comment" {
            break;
        }
        let text = normalize_comment(prev.text().as_ref());
        if text.is_empty() {
            break;
        }
        docs.push(text);
        current = prev.prev();
    }

    docs.reverse();
    docs.join("\n")
}

pub(super) fn extract_visibility<D: ast_grep_core::Doc>(node: &Node<D>) -> Visibility {
    for child in node.children() {
        if child.kind().as_ref() == "visibility_modifier" {
            let text = child.text().to_string();
            if text.contains("protected") {
                return Visibility::Protected;
            }
            if text.contains("private") {
                return Visibility::Private;
            }
            return Visibility::Public;
        }
    }
    Visibility::Public
}

pub(super) fn is_static<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    node.children()
        .any(|child| child.kind().as_ref() == "static_modifier")
}

pub(super) fn extract_attributes<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut lists = Vec::new();
    if let Some(attr_field) = node.field("attributes") {
        lists.push(attr_field);
    }
    lists.extend(
        node.children()
            .filter(|c| c.kind().as_ref() == "attribute_list"),
    );

    let mut attrs = Vec::new();
    for list in lists {
        for group in list
            .children()
            .filter(|c| c.kind().as_ref() == "attribute_group")
        {
            for attribute in group
                .children()
                .filter(|c| c.kind().as_ref() == "attribute")
            {
                let name = attribute
                    .children()
                    .find(|c| {
                        matches!(
                            c.kind().as_ref(),
                            "name" | "qualified_name" | "relative_name"
                        )
                    })
                    .map(|n| n.text().to_string())
                    .unwrap_or_default();
                if !name.is_empty() {
                    attrs.push(format!("attr:name:{name}"));
                    if let Some(args_node) = attribute.field("parameters") {
                        attrs.push(format!("attr:args:{}", args_node.text()));
                        for arg in args_node
                            .children()
                            .filter(|c| c.kind().as_ref() == "argument")
                        {
                            if let Some(named) = arg.field("name") {
                                let value = arg
                                    .children()
                                    .find(|c| c.kind().as_ref() != "name")
                                    .map(|v| v.text().to_string())
                                    .unwrap_or_default();
                                attrs.push(format!("attr:named:{}={value}", named.text()));
                            }
                        }
                    }
                }
            }
        }
    }
    attrs
}

pub(super) fn extract_parameter_descriptors<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(params) = node.field("parameters") else {
        return Vec::new();
    };

    params
        .children()
        .filter_map(|child| {
            let kind = child.kind();
            if !matches!(
                kind.as_ref(),
                "simple_parameter" | "variadic_parameter" | "property_promotion_parameter"
            ) {
                return None;
            }

            let name = child
                .field("name")
                .map(|n| n.text().to_string())
                .unwrap_or_default()
                .trim_start_matches('&')
                .trim_start_matches('$')
                .to_string();

            let mut desc = if let Some(ty) = types::normalize_type_node(child.field("type")) {
                if name.is_empty() {
                    ty
                } else {
                    format!("{name}: {ty}")
                }
            } else {
                name
            };

            let mut flags: Vec<String> = Vec::new();
            if kind.as_ref() == "variadic_parameter" {
                flags.push("variadic".to_string());
            }
            if child.field("reference_modifier").is_some() {
                flags.push("by_ref".to_string());
            }
            if child.field("default_value").is_some() {
                flags.push("default".to_string());
            }
            if kind.as_ref() == "property_promotion_parameter" {
                flags.push("promoted".to_string());
                if let Some(v) = child.field("visibility") {
                    flags.push(v.text().to_string());
                }
            }

            if !flags.is_empty() {
                desc = format!("{desc} [{}]", flags.join(","));
            }

            Some(desc)
        })
        .collect()
}

pub(super) fn owner_from_ancestors<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<(String, SymbolKind)> {
    let mut current = node.parent();
    while let Some(parent) = current {
        let owner_kind = match parent.kind().as_ref() {
            "class_declaration" | "anonymous_class" => SymbolKind::Class,
            "interface_declaration" => SymbolKind::Interface,
            "trait_declaration" => SymbolKind::Trait,
            "enum_declaration" => SymbolKind::Enum,
            _ => {
                current = parent.parent();
                continue;
            }
        };

        if parent.kind().as_ref() == "anonymous_class" {
            return Some((synthetic_name("anonymous_class", &parent), owner_kind));
        }

        if let Some(name) = parent.field("name") {
            return Some((name.text().to_string(), owner_kind));
        }

        current = parent.parent();
    }
    None
}

pub(super) fn extract_name_list<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .filter(|child| {
            matches!(
                child.kind().as_ref(),
                "name" | "qualified_name" | "relative_name" | "namespace_name"
            )
        })
        .map(|child| child.text().to_string())
        .collect()
}

pub(super) fn extract_type_bases<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut bases = Vec::new();
    bases.extend(
        node.children()
            .filter(|c| c.kind().as_ref() == "base_clause")
            .flat_map(|base| extract_name_list(&base)),
    );
    bases.extend(
        node.children()
            .filter(|c| c.kind().as_ref() == "class_interface_clause")
            .flat_map(|base| extract_name_list(&base)),
    );
    bases
        .into_iter()
        .map(|base| types::normalize_type_text(&base))
        .collect()
}

pub(super) fn collect_enum_variants<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .find(|c| c.kind().as_ref() == "enum_declaration_list")
        .map(|body| {
            body.children()
                .filter(|c| c.kind().as_ref() == "enum_case")
                .filter_map(|case| case.field("name").map(|n| n.text().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn apply_phpdoc_metadata(doc: &str, metadata: &mut SymbolMetadata) {
    let parsed = phpdoc::parse_phpdoc(doc);

    if metadata.return_type.is_none() {
        metadata.return_type = parsed.return_type.or_else(|| parsed.var_type.clone());
    }

    if !parsed.templates.is_empty() {
        metadata.type_parameters = Some(parsed.templates.join(", "));
    }

    if !parsed.extends.is_empty() || !parsed.implements.is_empty() {
        metadata.base_classes.extend(parsed.extends.clone());
        metadata.base_classes.extend(parsed.implements.clone());
    }

    metadata.attributes.extend(parsed.tags);

    if metadata.parameters.is_empty() {
        metadata.parameters.extend(
            parsed
                .param_types
                .iter()
                .map(|(name, ty)| format!("{name}: {ty}")),
        );
    } else {
        metadata.parameters = metadata
            .parameters
            .iter()
            .map(|param| {
                let param_name = param
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .split(' ')
                    .next()
                    .unwrap_or_default();

                parsed.param_types.get(param_name).map_or_else(
                    || param.clone(),
                    |doc_ty| {
                        if param.contains(':') {
                            param.clone()
                        } else {
                            format!("{param}: {doc_ty}")
                        }
                    },
                )
            })
            .collect();
    }
}

pub(super) fn assignment_target_alias<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind().as_ref() == "assignment_expression" {
            let left = parent.field("left").or_else(|| {
                parent.children().find(|c| {
                    matches!(
                        c.kind().as_ref(),
                        "variable_name" | "name" | "member_access_expression"
                    )
                })
            })?;
            return Some(left.text().to_string());
        }
        current = parent.parent();
    }
    None
}

pub(super) fn callable_context<D: ast_grep_core::Doc>(node: &Node<D>) -> (String, Option<String>) {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind().as_ref() {
            "pair" => {
                let alias = parent
                    .children()
                    .next()
                    .map(|key| key.text().to_string())
                    .filter(|s| !s.is_empty());
                return ("array_pair".to_string(), alias);
            }
            "assignment_expression" => {
                let alias = assignment_target_alias(node);
                return ("assignment".to_string(), alias);
            }
            "return_statement" => return ("return".to_string(), None),
            "argument" => return ("argument".to_string(), None),
            _ => current = parent.parent(),
        }
    }
    ("unknown".to_string(), None)
}

pub(super) fn owner_callable_from_ancestors<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<(String, SymbolKind)> {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind().as_ref() {
            "method_declaration" => {
                let name = parent.field("name")?.text().to_string();
                return Some((name, SymbolKind::Method));
            }
            "function_definition" => {
                let name = parent.field("name")?.text().to_string();
                return Some((name, SymbolKind::Function));
            }
            _ => current = parent.parent(),
        }
    }
    None
}

pub(super) fn synthetic_name<D: ast_grep_core::Doc>(kind: &str, node: &Node<D>) -> String {
    format!("<{kind}@L{}>", node.start_pos().line() as u32 + 1)
}

pub(super) fn normalize_php_name(path: &str) -> String {
    let trimmed = path.trim().trim_start_matches('\\');
    let parts: Vec<&str> = trimmed
        .split('\\')
        .filter(|segment| !segment.is_empty())
        .collect();
    parts.join("\\")
}

fn normalize_comment(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("/**") {
        return trimmed
            .trim_start_matches("/**")
            .trim_end_matches("*/")
            .lines()
            .map(|line| line.trim().trim_start_matches('*').trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    }
    if trimmed.starts_with("//") {
        return trimmed.trim_start_matches("//").trim().to_string();
    }
    if trimmed.starts_with('#') {
        return trimmed.trim_start_matches('#').trim().to_string();
    }
    trimmed.to_string()
}
