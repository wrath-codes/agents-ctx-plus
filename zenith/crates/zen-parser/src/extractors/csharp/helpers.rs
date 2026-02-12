use ast_grep_core::Node;

use crate::types::{SymbolKind, Visibility};

pub(super) fn extract_csharp_doc_before<D: ast_grep_core::Doc>(anchor: &Node<D>) -> String {
    let mut docs = Vec::new();
    let mut current = anchor.prev();
    while let Some(prev) = current {
        if prev.kind().as_ref() != "comment" {
            break;
        }
        let text = prev.text().trim().to_string();
        if !text.starts_with("///") {
            break;
        }
        docs.push(text.trim_start_matches("///").trim().to_string());
        current = prev.prev();
    }

    docs.reverse();
    docs.join("\n")
}

pub(super) fn extract_modifiers<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .filter(|child| child.kind().as_ref() == "modifier")
        .map(|child| child.text().to_string())
        .collect()
}

pub(super) fn visibility_from_modifiers(modifiers: &[String]) -> Visibility {
    if modifiers.iter().any(|m| m == "protected")
        && (modifiers.iter().any(|m| m == "private") || modifiers.iter().any(|m| m == "internal"))
    {
        return Visibility::Protected;
    }
    if modifiers.iter().any(|m| m == "public") {
        return Visibility::Public;
    }
    if modifiers.iter().any(|m| m == "private") {
        return Visibility::Private;
    }
    if modifiers.iter().any(|m| m == "internal") {
        if modifiers.iter().any(|m| m == "protected") {
            return Visibility::Protected;
        }
        return Visibility::PublicCrate;
    }
    if modifiers.iter().any(|m| m == "protected") {
        return Visibility::Protected;
    }
    Visibility::Private
}

pub(super) fn is_static_member(modifiers: &[String]) -> bool {
    modifiers.iter().any(|m| m == "static")
}

pub(super) fn is_const_member(modifiers: &[String]) -> bool {
    modifiers.iter().any(|m| m == "const")
}

pub(super) fn owner_from_ancestors<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<(String, SymbolKind)> {
    let mut current = node.parent();
    while let Some(parent) = current {
        let kind = parent.kind();
        let symbol_kind = match kind.as_ref() {
            "class_declaration" | "record_declaration" => SymbolKind::Class,
            "struct_declaration" => SymbolKind::Struct,
            "interface_declaration" => SymbolKind::Interface,
            _ => {
                current = parent.parent();
                continue;
            }
        };

        if let Some(name) = parent.field("name").map(|n| n.text().to_string()) {
            return Some((name, symbol_kind));
        }
        current = parent.parent();
    }
    None
}

pub(super) fn extract_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(params) = node.field("parameters").or_else(|| node.field("parameter")) else {
        return Vec::new();
    };

    params
        .children()
        .filter(|child| {
            let kind = child.kind();
            let k = kind.as_ref();
            k.contains("parameter") || k == "parameter" || k == "arg_declaration"
        })
        .map(|child| child.text().to_string())
        .collect()
}

pub(super) fn extract_base_types<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .find(|child| child.kind().as_ref() == "base_list")
        .map(|base_list| {
            base_list
                .text()
                .trim_start_matches(':')
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn extract_variable_names_from_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<String> {
    let Some(var_decl) = node
        .children()
        .find(|child| child.kind().as_ref() == "variable_declaration")
    else {
        return Vec::new();
    };

    var_decl
        .children()
        .filter(|child| child.kind().as_ref() == "variable_declarator")
        .filter_map(|declarator| declarator.field("name").map(|name| name.text().to_string()))
        .collect()
}
