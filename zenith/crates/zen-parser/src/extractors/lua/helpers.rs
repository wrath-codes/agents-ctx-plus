use ast_grep_core::Node;

use crate::types::{SymbolKind, SymbolMetadata, Visibility};

pub(super) struct MemberName {
    pub owner: String,
    pub member: String,
    pub is_method_syntax: bool,
    pub access_kind: String,
}

pub(super) fn extract_lua_doc_before<D: ast_grep_core::Doc>(anchor: &Node<D>) -> String {
    let mut lines = Vec::new();
    let mut current = anchor.prev();

    while let Some(prev) = current {
        let kind = prev.kind();
        if kind.as_ref() != "comment" {
            break;
        }

        let text = normalize_comment(prev.text().as_ref());
        if text.is_empty() {
            break;
        }

        lines.push(text);
        current = prev.prev();
    }

    lines.reverse();
    lines.join("\n")
}

pub(super) fn extract_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(parameters) = node.field("parameters") else {
        return Vec::new();
    };

    parameters
        .children()
        .filter_map(|child| {
            let kind = child.kind();
            match kind.as_ref() {
                "identifier" | "vararg_expression" => Some(child.text().to_string()),
                _ => None,
            }
        })
        .collect()
}

pub(super) fn extract_member_name<D: ast_grep_core::Doc>(name: &Node<D>) -> Option<MemberName> {
    let kind = name.kind();
    match kind.as_ref() {
        "dot_index_expression" => Some(MemberName {
            owner: name.field("table")?.text().to_string(),
            member: name.field("field")?.text().to_string(),
            is_method_syntax: false,
            access_kind: "dot".to_string(),
        }),
        "method_index_expression" => Some(MemberName {
            owner: name.field("table")?.text().to_string(),
            member: name.field("method")?.text().to_string(),
            is_method_syntax: true,
            access_kind: "colon".to_string(),
        }),
        "bracket_index_expression" => {
            let field = name.field("field")?;
            let member = normalize_member_key(&field)?;
            Some(MemberName {
                owner: name.field("table")?.text().to_string(),
                member,
                is_method_syntax: false,
                access_kind: "bracket".to_string(),
            })
        }
        _ => None,
    }
}

pub(super) fn normalize_member_key<D: ast_grep_core::Doc>(field: &Node<D>) -> Option<String> {
    let kind = field.kind();
    match kind.as_ref() {
        "identifier" => Some(field.text().to_string()),
        "string" => {
            let text = field.text();
            let trimmed = text.trim();
            Some(
                trimmed
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim_start_matches("[[")
                    .trim_end_matches("]]")
                    .to_string(),
            )
        }
        _ => None,
    }
}

pub(super) fn apply_luadoc_metadata(doc: &str, metadata: &mut SymbolMetadata) {
    if doc.is_empty() {
        return;
    }

    let mut param_types: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut luadoc_params: Vec<String> = Vec::new();

    for line in doc.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('@') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@param ") {
            let mut parts = rest.split_whitespace();
            let name = parts.next().unwrap_or_default();
            let ty = parts.next().unwrap_or_default();
            if !name.is_empty() {
                if ty.is_empty() {
                    luadoc_params.push(name.to_string());
                    metadata.attributes.push(format!("luadoc:param:{name}"));
                } else {
                    param_types.insert(name.to_string(), ty.to_string());
                    luadoc_params.push(format!("{name}: {ty}"));
                    metadata
                        .attributes
                        .push(format!("luadoc:param:{name}:{ty}"));
                }
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@return ") {
            let ret = rest.split_whitespace().next().unwrap_or_default().trim();
            if !ret.is_empty() && metadata.return_type.is_none() {
                metadata.return_type = Some(ret.to_string());
            }
            metadata.attributes.push(format!("luadoc:return:{ret}"));
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@class ") {
            let class_name = rest.split_whitespace().next().unwrap_or_default().trim();
            if !class_name.is_empty() {
                metadata
                    .attributes
                    .push(format!("luadoc:class:{class_name}"));
                continue;
            }
        }

        if let Some(rest) = trimmed.strip_prefix("@field ") {
            let mut parts = rest.split_whitespace();
            let field_name = parts.next().unwrap_or_default().trim();
            let field_ty = parts.next().unwrap_or_default().trim();
            if !field_name.is_empty() && !field_ty.is_empty() {
                metadata
                    .attributes
                    .push(format!("luadoc:field:{field_name}:{field_ty}"));
                continue;
            }
            if !field_name.is_empty() {
                metadata
                    .attributes
                    .push(format!("luadoc:field:{field_name}"));
                continue;
            }
        }

        if let Some(rest) = trimmed.strip_prefix("@type ") {
            let ty = rest.trim();
            if !ty.is_empty() {
                metadata.attributes.push(format!("luadoc:type:{ty}"));
                continue;
            }
        }

        metadata.attributes.push(format!("luadoc:{trimmed}"));
    }

    if metadata.parameters.is_empty() {
        metadata.parameters = luadoc_params;
        return;
    }

    metadata.parameters = metadata
        .parameters
        .iter()
        .map(|param| {
            param_types
                .get(param)
                .map_or_else(|| param.clone(), |ty| format!("{param}: {ty}"))
        })
        .collect();
}

pub(super) fn add_local_attrs(metadata: &mut SymbolMetadata, attrs: &[String]) {
    metadata.attributes.extend(
        attrs
            .iter()
            .filter(|a| !a.is_empty())
            .map(|a| format!("local_attr:{a}")),
    );
}

pub(super) const fn visibility_for_local(is_local: bool) -> Visibility {
    if is_local {
        Visibility::Private
    } else {
        Visibility::Public
    }
}

pub(super) const fn owner_kind_for_table() -> SymbolKind {
    SymbolKind::Module
}

fn normalize_comment(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("--[[") {
        return trimmed
            .trim_start_matches("--[[")
            .trim_end_matches("]]")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
    }
    if trimmed.starts_with("---") {
        return trimmed.trim_start_matches("---").trim().to_string();
    }
    trimmed.trim_start_matches("--").trim().to_string()
}
