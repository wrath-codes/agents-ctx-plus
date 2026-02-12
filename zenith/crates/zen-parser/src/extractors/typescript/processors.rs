use ast_grep_core::Node;

use crate::extractors::helpers;
use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, TypeScriptMetadataExt, Visibility};

use super::ts_helpers::{
    extract_jsdoc_before, extract_ts_parameters, extract_ts_return_type, parse_jsdoc_sections,
};

// ── export_statement unwrapping ────────────────────────────────────

pub(super) fn process_export_statement<D: ast_grep_core::Doc>(
    export_node: &Node<D>,
) -> Vec<ParsedItem> {
    let is_default = export_node
        .children()
        .any(|c| c.kind().as_ref() == "default");

    let mut items = Vec::new();
    for child in export_node.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_declaration" => {
                if let Some(item) = process_function(&child, export_node, true, is_default) {
                    items.push(item);
                }
            }
            "class_declaration" | "abstract_class_declaration" => {
                if let Some(item) = process_class(&child, export_node, true, is_default) {
                    items.push(item);
                }
            }
            "interface_declaration" => {
                if let Some(item) = process_interface(&child, export_node, true) {
                    items.push(item);
                }
            }
            "type_alias_declaration" => {
                if let Some(item) = process_type_alias(&child, export_node, true) {
                    items.push(item);
                }
            }
            "enum_declaration" => {
                if let Some(item) = process_enum(&child, export_node, true) {
                    items.push(item);
                }
            }
            "lexical_declaration" => {
                items.extend(process_lexical_declaration(&child, export_node, true));
            }
            "internal_module" => {
                if let Some(item) = process_namespace(&child, export_node, true) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }
    items
}

// ── function_declaration ───────────────────────────────────────────

pub(super) fn process_function<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
    is_default: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let is_async = node.children().any(|c| c.kind().as_ref() == "async");
    let return_type = extract_ts_return_type(node);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_async {
        metadata.mark_async();
    }
    if is_exported {
        metadata.mark_exported();
    }
    if is_default {
        metadata.mark_default_export();
    }
    metadata.set_return_type(return_type);
    metadata.set_type_parameters(type_params);
    metadata.set_parameters(extract_ts_parameters(node));
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

// ── class_declaration / abstract_class_declaration ─────────────────

pub(super) fn process_class<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
    is_default: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    let is_abstract = node.kind().as_ref() == "abstract_class_declaration";

    let extends = extract_class_heritage(node, "extends");
    let implements = extract_class_heritage(node, "implements");
    let (methods, fields) = extract_class_members(node);

    let is_error_type =
        helpers::is_error_type_by_name(&name) || extends.iter().any(|e| e == "Error");

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_exported {
        metadata.mark_exported();
    }
    if is_default {
        metadata.mark_default_export();
    }
    metadata.set_type_parameters(type_params);
    metadata.set_base_classes(extends);
    metadata.set_implements(implements);
    metadata.set_methods(methods);
    metadata.set_fields(fields);
    if is_error_type {
        metadata.mark_error_type();
    }
    if is_abstract {
        metadata.mark_unsafe();
    }
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Class,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

fn extract_class_heritage<D: ast_grep_core::Doc>(node: &Node<D>, clause_kind: &str) -> Vec<String> {
    let target = match clause_kind {
        "extends" => "extends_clause",
        "implements" => "implements_clause",
        _ => return Vec::new(),
    };
    for child in node.children() {
        if child.kind().as_ref() == "class_heritage" {
            for clause in child.children() {
                if clause.kind().as_ref() == target {
                    return clause
                        .children()
                        .filter(|c| {
                            let k = c.kind();
                            k.as_ref() != "extends"
                                && k.as_ref() != "implements"
                                && k.as_ref() != ","
                        })
                        .map(|c| c.text().to_string())
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

fn extract_class_members<D: ast_grep_core::Doc>(node: &Node<D>) -> (Vec<String>, Vec<String>) {
    let mut methods = Vec::new();
    let mut fields = Vec::new();

    let Some(body) = node.field("body") else {
        return (methods, fields);
    };

    for child in body.children() {
        let k = child.kind();
        match k.as_ref() {
            "method_definition" => {
                if let Some(name) = child.field("name").map(|n| n.text().to_string()) {
                    methods.push(name);
                }
            }
            "public_field_definition" => {
                if let Some(name) = child.field("name").map(|n| n.text().to_string()) {
                    fields.push(name);
                }
            }
            "abstract_method_definition" | "abstract_method_signature" => {
                if let Some(name) = child.field("name").map(|n| n.text().to_string()) {
                    methods.push(name);
                }
            }
            _ => {}
        }
    }
    (methods, fields)
}

// ── interface_declaration ──────────────────────────────────────────

pub(super) fn process_interface<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    let extends = extract_interface_heritage(node);
    let members = extract_interface_members(node);

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_exported {
        metadata.mark_exported();
    }
    metadata.set_type_parameters(type_params);
    metadata.set_base_classes(extends);
    metadata.set_methods(members);
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Interface,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

fn extract_interface_heritage<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    for child in node.children() {
        if child.kind().as_ref() == "extends_type_clause" {
            return child
                .children()
                .filter(|c| c.kind().as_ref() != "extends" && c.kind().as_ref() != ",")
                .map(|c| c.text().to_string())
                .collect();
        }
    }
    Vec::new()
}

fn extract_interface_members<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut members = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "interface_body" || child.kind().as_ref() == "object_type" {
            for member in child.children() {
                let mk = member.kind();
                if (mk.as_ref() == "method_signature" || mk.as_ref() == "property_signature")
                    && let Some(name) = member.field("name").map(|n| n.text().to_string())
                {
                    members.push(name);
                }
            }
        }
    }
    members
}

// ── type_alias_declaration ─────────────────────────────────────────

pub(super) fn process_type_alias<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_exported {
        metadata.mark_exported();
    }
    metadata.set_type_parameters(type_params);
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::TypeAlias,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

// ── enum_declaration ───────────────────────────────────────────────

pub(super) fn process_enum<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let variants = extract_enum_members(node);

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    metadata.set_variants(variants);
    if is_exported {
        metadata.mark_exported();
    }
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Enum,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

fn extract_enum_members<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut variants = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "enum_body" {
            for member in child.children() {
                if member.kind().as_ref() == "enum_assignment"
                    || member.kind().as_ref() == "property_identifier"
                {
                    let name = member
                        .field("name")
                        .map_or_else(|| member.text().to_string(), |n| n.text().to_string());
                    if !name.is_empty() && name != "," && name != "{" && name != "}" {
                        variants.push(name);
                    }
                }
            }
        }
    }
    variants
}

// ── namespace (internal_module) ─────────────────────────────────────

pub(super) fn process_namespace<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    let mut metadata = SymbolMetadata::default();
    if is_exported {
        metadata.mark_exported();
    }
    metadata.set_doc_sections(doc_sections);

    Some(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature: format!(
            "namespace {}",
            node.field("name")
                .map(|n| n.text().to_string())
                .unwrap_or_default()
        ),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    })
}

// ── ambient_declaration (declare ...) ──────────────────────────────

pub(super) fn process_ambient_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    for child in node.children() {
        let k = child.kind();
        match k.as_ref() {
            "function_signature" => {
                if let Some(item) = process_function_signature(&child, node) {
                    items.push(item);
                }
            }
            "lexical_declaration" => {
                items.extend(process_lexical_declaration(&child, node, false));
            }
            "class_declaration" => {
                if let Some(item) = process_class(&child, node, false, false) {
                    items.push(item);
                }
            }
            "module" => {
                if let Some(name_node) = child.field("name") {
                    let name = name_node.text().to_string();
                    items.push(ParsedItem {
                        kind: SymbolKind::Module,
                        name: name.trim_matches('"').to_string(),
                        signature: format!("declare module {name}"),
                        source: helpers::extract_source(&child, 50),
                        doc_comment: String::new(),
                        start_line: node.start_pos().line() as u32 + 1,
                        end_line: node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Export,
                        metadata: SymbolMetadata::default(),
                    });
                }
            }
            _ => {}
        }
    }
    items
}

// ── function_signature (overloads, declare function) ───────────────

pub(super) fn process_function_signature<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let return_type = extract_ts_return_type(node);
    let type_params = node
        .field("type_parameters")
        .map(|tp| tp.text().to_string());

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Export,
        metadata: {
            let mut metadata = SymbolMetadata::default();
            metadata.mark_exported();
            metadata.set_return_type(return_type);
            metadata.set_type_parameters(type_params);
            metadata.set_parameters(extract_ts_parameters(node));
            metadata.set_doc_sections(doc_sections);
            metadata
        },
    })
}

// ── variable_declaration (var/let) ─────────────────────────────────

pub(super) fn process_variable_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "variable_declarator"
            && let Some(item) = process_variable_declarator(&child, node, jsdoc_anchor, is_exported)
        {
            items.push(item);
        }
    }
    items
}

// ── lexical_declaration (arrow functions, consts) ──────────────────

pub(super) fn process_lexical_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    for child in node.children() {
        if child.kind().as_ref() == "variable_declarator"
            && let Some(item) = process_variable_declarator(&child, node, jsdoc_anchor, is_exported)
        {
            items.push(item);
        }
    }
    items
}

fn process_variable_declarator<D: ast_grep_core::Doc>(
    declarator: &Node<D>,
    declaration: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = declarator.field("name").map(|n| n.text().to_string())?;

    let value = declarator.field("value");
    let is_arrow = value
        .as_ref()
        .is_some_and(|v| v.kind().as_ref() == "arrow_function");

    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    if is_arrow {
        let arrow = value.unwrap();
        let is_async = arrow.children().any(|c| c.kind().as_ref() == "async");
        let return_type = extract_ts_return_type(&arrow);
        let params = extract_ts_parameters(&arrow);
        let type_params = arrow
            .field("type_parameters")
            .map(|tp| tp.text().to_string());

        Some(ParsedItem {
            kind: SymbolKind::Function,
            name,
            signature: helpers::extract_signature(declaration),
            source: helpers::extract_source(declaration, 50),
            doc_comment: jsdoc,
            start_line: declaration.start_pos().line() as u32 + 1,
            end_line: declaration.end_pos().line() as u32 + 1,
            visibility,
            metadata: {
                let mut metadata = SymbolMetadata::default();
                if is_async {
                    metadata.mark_async();
                }
                if is_exported {
                    metadata.mark_exported();
                }
                metadata.set_return_type(return_type);
                metadata.set_type_parameters(type_params);
                metadata.set_parameters(params);
                metadata.set_doc_sections(doc_sections);
                metadata
            },
        })
    } else {
        let type_annotation = declarator
            .children()
            .find(|c| c.kind().as_ref() == "type_annotation")
            .map(|ta| {
                ta.text()
                    .to_string()
                    .trim_start_matches(':')
                    .trim()
                    .to_string()
            });

        Some(ParsedItem {
            kind: SymbolKind::Const,
            name,
            signature: helpers::extract_signature(declaration),
            source: helpers::extract_source(declaration, 50),
            doc_comment: jsdoc,
            start_line: declaration.start_pos().line() as u32 + 1,
            end_line: declaration.end_pos().line() as u32 + 1,
            visibility,
            metadata: {
                let mut metadata = SymbolMetadata::default();
                if is_exported {
                    metadata.mark_exported();
                }
                metadata.set_return_type(type_annotation);
                metadata.set_doc_sections(doc_sections);
                metadata
            },
        })
    }
}
