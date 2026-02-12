use ast_grep_core::Node;

use crate::types::{SymbolKind, Visibility};

pub(super) struct ModuleDirectiveParts {
    pub directive: String,
    pub subject: Option<String>,
    pub targets: Vec<String>,
    pub modifiers: Vec<String>,
}

pub(super) fn extract_javadoc_before<D: ast_grep_core::Doc>(anchor: &Node<D>) -> String {
    let mut docs = Vec::new();
    let mut current = anchor.prev();
    while let Some(prev) = current {
        let kind = prev.kind();
        if kind.as_ref() == "line_comment" {
            current = prev.prev();
            continue;
        }
        if kind.as_ref() != "block_comment" {
            break;
        }

        let text = prev.text().trim().to_string();
        if !text.starts_with("/**") || text.starts_with("/***") {
            break;
        }

        let inner = text
            .trim_start_matches("/**")
            .trim_end_matches("*/")
            .lines()
            .map(|line| line.trim().trim_start_matches('*').trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if inner.is_empty() {
            break;
        }

        docs.push(inner);
        break;
    }

    docs.reverse();
    docs.join("\n")
}

pub(super) fn extract_modifiers<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    const JAVA_MODIFIERS: &[&str] = &[
        "public",
        "protected",
        "private",
        "static",
        "final",
        "abstract",
        "native",
        "synchronized",
        "strictfp",
        "transient",
        "volatile",
        "default",
        "sealed",
        "non-sealed",
    ];

    let Some(modifiers) = node.children().find(|c| c.kind().as_ref() == "modifiers") else {
        return Vec::new();
    };

    modifiers
        .text()
        .split_whitespace()
        .filter(|token| JAVA_MODIFIERS.contains(token))
        .map(ToString::to_string)
        .collect()
}

pub(super) fn visibility_from_modifiers(modifiers: &[String]) -> Visibility {
    if modifiers.iter().any(|m| m == "public") {
        return Visibility::Public;
    }
    if modifiers.iter().any(|m| m == "protected") {
        return Visibility::Protected;
    }
    if modifiers.iter().any(|m| m == "private") {
        return Visibility::Private;
    }
    Visibility::PublicCrate
}

pub(super) fn is_static_member(modifiers: &[String]) -> bool {
    modifiers.iter().any(|m| m == "static")
}

pub(super) fn is_const_member(modifiers: &[String]) -> bool {
    modifiers.iter().any(|m| m == "static") && modifiers.iter().any(|m| m == "final")
}

pub(super) fn extract_annotations<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(modifiers) = node.children().find(|c| c.kind().as_ref() == "modifiers") else {
        return Vec::new();
    };

    modifiers
        .children()
        .filter(|child| {
            let kind = child.kind();
            kind.as_ref() == "annotation" || kind.as_ref() == "marker_annotation"
        })
        .map(|child| child.text().to_string())
        .collect()
}

pub(super) fn extract_throws<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.children()
        .find(|child| child.kind().as_ref() == "throws")
        .map(|throws| throws.text().to_string())
}

pub(super) fn extract_record_components<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<(String, Option<String>)> {
    let Some(parameters) = node.field("parameters") else {
        return Vec::new();
    };

    parameters
        .children()
        .filter(|child| child.kind().as_ref() == "formal_parameter")
        .filter_map(|child| {
            let name = child.field("name")?.text().to_string();
            let type_name = child.field("type").map(|t| t.text().to_string());
            Some((name, type_name))
        })
        .collect()
}

pub(super) fn owner_from_ancestors<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<(String, SymbolKind)> {
    let mut current = node.parent();
    while let Some(parent) = current {
        let owner_kind = match parent.kind().as_ref() {
            "class_declaration" => SymbolKind::Class,
            "interface_declaration" | "annotation_type_declaration" => SymbolKind::Interface,
            "enum_declaration" => SymbolKind::Enum,
            "record_declaration" => SymbolKind::Struct,
            _ => {
                current = parent.parent();
                continue;
            }
        };

        if let Some(name) = parent.field("name").map(|n| n.text().to_string()) {
            return Some((name, owner_kind));
        }
        current = parent.parent();
    }
    None
}

pub(super) fn extract_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(parameters) = node.field("parameters") else {
        return Vec::new();
    };

    parameters
        .children()
        .filter(|child| {
            let kind = child.kind();
            matches!(
                kind.as_ref(),
                "formal_parameter" | "receiver_parameter" | "spread_parameter"
            )
        })
        .map(|child| child.text().to_string())
        .collect()
}

pub(super) fn extract_variable_names_from_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<String> {
    node.children()
        .filter(|child| child.kind().as_ref() == "variable_declarator")
        .filter_map(|declarator| declarator.field("name").map(|name| name.text().to_string()))
        .collect()
}

pub(super) fn extract_base_types<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut out = Vec::new();

    if let Some(superclass) = node.field("superclass") {
        out.extend(split_type_clause(superclass.text().as_ref()));
    }
    if let Some(interfaces) = node.field("interfaces") {
        out.extend(split_type_clause(interfaces.text().as_ref()));
    }

    out.extend(
        node.children()
            .filter(|child| child.kind().as_ref() == "extends_interfaces")
            .flat_map(|child| split_type_clause(child.text().as_ref())),
    );

    out
}

pub(super) fn extract_enum_variants<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .find(|child| child.kind().as_ref() == "enum_body")
        .map(|body| {
            body.children()
                .filter(|child| child.kind().as_ref() == "enum_constant")
                .filter_map(|variant| variant.field("name").map(|name| name.text().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn split_type_clause(clause: &str) -> Vec<String> {
    clause
        .trim()
        .trim_start_matches("extends")
        .trim_start_matches("implements")
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(super) fn extract_module_directive_parts<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ModuleDirectiveParts> {
    let directive = match node.kind().as_ref() {
        "requires_module_directive" => "requires",
        "exports_module_directive" => "exports",
        "opens_module_directive" => "opens",
        "uses_module_directive" => "uses",
        "provides_module_directive" => "provides",
        _ => return None,
    }
    .to_string();

    let names = scoped_names(node);
    let mut targets = Vec::new();
    let mut modifiers = Vec::new();

    let subject = match directive.as_str() {
        "requires" => {
            for child in node.children() {
                if child.kind().as_ref() == "requires_modifier" {
                    modifiers.push(child.text().to_string());
                }
            }
            node.field("module")
                .map(|n| n.text().to_string())
                .or_else(|| names.first().cloned())
        }
        "exports" | "opens" => {
            let subject = node
                .field("package")
                .map(|n| n.text().to_string())
                .or_else(|| names.first().cloned());
            if let Some(ref package_name) = subject {
                targets = names
                    .into_iter()
                    .filter(|name| name != package_name)
                    .collect();
            }
            subject
        }
        "uses" => node
            .field("type")
            .map(|n| n.text().to_string())
            .or_else(|| names.first().cloned()),
        "provides" => {
            let subject = node
                .field("provided")
                .map(|n| n.text().to_string())
                .or_else(|| names.first().cloned());
            if let Some(ref provided) = subject {
                targets = names.into_iter().filter(|name| name != provided).collect();
            }
            subject
        }
        _ => None,
    };

    Some(ModuleDirectiveParts {
        directive,
        subject,
        targets,
        modifiers,
    })
}

fn scoped_names<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .filter(|child| {
            let kind = child.kind();
            let k = kind.as_ref();
            k == "identifier" || k == "scoped_identifier"
        })
        .map(|child| child.text().to_string())
        .collect()
}
