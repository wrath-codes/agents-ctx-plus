//! TypeScript/JavaScript/TSX rich extractor.
//!
//! Shared extractor for TypeScript, JavaScript, and TSX. Extracts
//! functions, classes, interfaces, type aliases, enums, and arrow
//! functions with `JSDoc` support and export detection.

use ast_grep_core::Node;
use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use super::helpers;
use crate::types::{
    DocSections, ParsedItem, SymbolKind, SymbolMetadata, TypeScriptMetadataExt, Visibility,
};

const TS_TOP_KINDS: &[&str] = &[
    "export_statement",
    "function_declaration",
    "class_declaration",
    "abstract_class_declaration",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
    "lexical_declaration",
    "variable_declaration",
    "ambient_declaration",
    "internal_module",
    "function_signature",
];

/// Extract all API symbols from a TypeScript/JavaScript/TSX source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    lang: SupportLang,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = TS_TOP_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, lang))
        .collect();
    let matcher = Any::new(matchers);

    for node in root.root().find_all(&matcher) {
        let kind = node.kind();
        match kind.as_ref() {
            "export_statement" => {
                items.extend(process_export_statement(&node));
            }
            "function_declaration" => {
                if let Some(item) = process_function(&node, &node, false, false) {
                    items.push(item);
                }
            }
            "class_declaration" | "abstract_class_declaration" => {
                if let Some(item) = process_class(&node, &node, false, false) {
                    items.push(item);
                }
            }
            "interface_declaration" => {
                if let Some(item) = process_interface(&node, &node, false) {
                    items.push(item);
                }
            }
            "type_alias_declaration" => {
                if let Some(item) = process_type_alias(&node, &node, false) {
                    items.push(item);
                }
            }
            "enum_declaration" => {
                if let Some(item) = process_enum(&node, &node, false) {
                    items.push(item);
                }
            }
            "lexical_declaration" => {
                items.extend(process_lexical_declaration(&node, &node, false));
            }
            "variable_declaration" => {
                items.extend(process_variable_declaration(&node, &node, false));
            }
            "ambient_declaration" => {
                items.extend(process_ambient_declaration(&node));
            }
            "internal_module" => {
                if let Some(item) = process_namespace(&node, &node, false) {
                    items.push(item);
                }
            }
            "function_signature" => {
                if let Some(item) = process_function_signature(&node, &node) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }
    Ok(items)
}

// ── export_statement unwrapping ────────────────────────────────────

fn process_export_statement<D: ast_grep_core::Doc>(export_node: &Node<D>) -> Vec<ParsedItem> {
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

fn process_function<D: ast_grep_core::Doc>(
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

fn process_class<D: ast_grep_core::Doc>(
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

fn process_interface<D: ast_grep_core::Doc>(
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

fn process_type_alias<D: ast_grep_core::Doc>(
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

fn process_enum<D: ast_grep_core::Doc>(
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

fn process_namespace<D: ast_grep_core::Doc>(
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

fn process_ambient_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
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

fn process_function_signature<D: ast_grep_core::Doc>(
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

fn process_variable_declaration<D: ast_grep_core::Doc>(
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

fn process_lexical_declaration<D: ast_grep_core::Doc>(
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

// ── JSDoc extraction ───────────────────────────────────────────────

fn extract_jsdoc_before<D: ast_grep_core::Doc>(anchor: &Node<D>) -> String {
    if let Some(prev) = anchor.prev()
        && prev.kind().as_ref() == "comment"
    {
        let text = prev.text().to_string();
        if text.starts_with("/**") {
            return parse_jsdoc_text(&text);
        }
    }
    String::new()
}

fn parse_jsdoc_text(text: &str) -> String {
    let text = text.trim_start_matches("/**").trim_end_matches("*/").trim();
    text.lines()
        .map(|line| {
            let trimmed = line.trim();
            let stripped = trimmed.trim_start_matches('*');
            stripped.strip_prefix(' ').unwrap_or(stripped)
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn parse_jsdoc_sections(doc: &str) -> DocSections {
    let mut sections = DocSections::default();
    for line in doc.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@param ") {
            if let Some((name, desc)) = rest.split_once(' ') {
                sections
                    .args
                    .insert(name.to_string(), desc.trim().to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("@returns ") {
            sections.returns = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("@throws ") {
            let (exc, desc) = rest.split_once(' ').unwrap_or((rest, ""));
            sections
                .raises
                .insert(exc.to_string(), desc.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("@example") {
            let content = rest.trim();
            if content.is_empty() {
                sections.examples = Some(String::new());
            } else {
                sections.examples = Some(content.to_string());
            }
        }
    }
    sections
}

// ── TS-specific helpers ────────────────────────────────────────────

fn extract_ts_return_type<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("return_type")
        .map(|rt| {
            rt.text()
                .to_string()
                .trim_start_matches(':')
                .trim()
                .to_string()
        })
        .or_else(|| {
            node.children()
                .find(|c| c.kind().as_ref() == "type_annotation")
                .map(|ta| {
                    ta.text()
                        .to_string()
                        .trim_start_matches(':')
                        .trim()
                        .to_string()
                })
        })
}

fn extract_ts_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(params) = node.field("parameters") else {
        return Vec::new();
    };
    params
        .children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "required_parameter"
                || k.as_ref() == "optional_parameter"
                || k.as_ref() == "rest_parameter"
        })
        .map(|c| c.text().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SymbolKind;
    use ast_grep_language::LanguageExt;
    use pretty_assertions::assert_eq;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::TypeScript.ast_grep(source);
        extract(&root, SupportLang::TypeScript).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("should find item named '{name}'"))
    }

    #[test]
    fn exported_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Export);
        assert!(f.metadata.is_async);
        assert!(f.metadata.is_exported);
    }

    #[test]
    fn non_exported_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "internalHelper");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Private);
        assert!(!f.metadata.is_exported);
    }

    #[test]
    fn default_export_class() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "EventEmitter");
        assert_eq!(c.kind, SymbolKind::Class);
        assert_eq!(c.visibility, Visibility::Export);
        assert!(c.metadata.is_default_export);
        assert!(c.metadata.is_exported);
    }

    #[test]
    fn class_methods_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "EventEmitter");
        assert!(c.metadata.methods.contains(&"on".to_string()));
        assert!(c.metadata.methods.contains(&"emit".to_string()));
        assert!(c.metadata.methods.contains(&"cleanup".to_string()));
    }

    #[test]
    fn error_class_detected() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "HttpError");
        assert_eq!(c.kind, SymbolKind::Class);
        assert!(c.metadata.is_error_type);
        assert!(c.metadata.base_classes.contains(&"Error".to_string()));
    }

    #[test]
    fn exported_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Handler");
        assert_eq!(i.kind, SymbolKind::Interface);
        assert_eq!(i.visibility, Visibility::Export);
        assert!(i.metadata.is_exported);
    }

    #[test]
    fn non_exported_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "PrivateConfig");
        assert_eq!(i.kind, SymbolKind::Interface);
        assert_eq!(i.visibility, Visibility::Private);
    }

    #[test]
    fn interface_members_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "Handler");
        assert!(i.metadata.methods.contains(&"handle".to_string()));
        assert!(i.metadata.methods.contains(&"name".to_string()));
    }

    #[test]
    fn exported_type_alias_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "Result");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
        assert_eq!(t.visibility, Visibility::Export);
    }

    #[test]
    fn non_exported_type_alias_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "InternalState");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
        assert_eq!(t.visibility, Visibility::Private);
    }

    #[test]
    fn exported_enum_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let e = find_by_name(&items, "Color");
        assert_eq!(e.kind, SymbolKind::Enum);
        assert_eq!(e.visibility, Visibility::Export);
    }

    #[test]
    fn non_exported_enum_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let e = find_by_name(&items, "InternalStatus");
        assert_eq!(e.kind, SymbolKind::Enum);
        assert_eq!(e.visibility, Visibility::Private);
    }

    #[test]
    fn enum_variants_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let e = find_by_name(&items, "Color");
        assert!(
            e.metadata.variants.contains(&"Red".to_string()),
            "variants: {:?}",
            e.metadata.variants
        );
        assert!(
            e.metadata.variants.contains(&"Green".to_string()),
            "variants: {:?}",
            e.metadata.variants
        );
        assert!(
            e.metadata.variants.contains(&"Blue".to_string()),
            "variants: {:?}",
            e.metadata.variants
        );
    }

    #[test]
    fn exported_arrow_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "fetchData");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Export);
        assert!(f.metadata.is_async);
    }

    #[test]
    fn non_async_arrow_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "add");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Export);
        assert!(!f.metadata.is_async);
    }

    #[test]
    fn non_exported_arrow_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "multiply");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Private);
    }

    #[test]
    fn const_declaration_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "MAX_RETRIES");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.visibility, Visibility::Export);
    }

    #[test]
    fn non_exported_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "INTERNAL_TIMEOUT");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.visibility, Visibility::Private);
    }

    #[test]
    fn abstract_class_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Shape");
        assert_eq!(c.kind, SymbolKind::Class);
        assert_eq!(c.visibility, Visibility::Export);
        assert!(c.metadata.is_unsafe, "is_unsafe used for abstract");
    }

    #[test]
    fn abstract_class_methods_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Shape");
        assert!(
            c.metadata.methods.contains(&"area".to_string()),
            "methods: {:?}",
            c.metadata.methods
        );
        assert!(
            c.metadata.methods.contains(&"perimeter".to_string()),
            "methods: {:?}",
            c.metadata.methods
        );
    }

    #[test]
    fn jsdoc_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert!(
            f.doc_comment.contains("Process a list of items"),
            "doc: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn jsdoc_params_parsed() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert!(
            f.metadata.doc_sections.args.contains_key("items"),
            "args: {:?}",
            f.metadata.doc_sections.args
        );
        assert!(
            f.metadata.doc_sections.args.contains_key("handler"),
            "args: {:?}",
            f.metadata.doc_sections.args
        );
    }

    #[test]
    fn jsdoc_returns_parsed() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert!(
            f.metadata.doc_sections.returns.is_some(),
            "should have @returns"
        );
    }

    #[test]
    fn jsdoc_throws_parsed() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert!(
            f.metadata.doc_sections.raises.contains_key("Error"),
            "raises: {:?}",
            f.metadata.doc_sections.raises
        );
    }

    #[test]
    fn signature_no_body_leak() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        for item in &items {
            if !item.signature.is_empty() && item.kind != SymbolKind::Const {
                assert!(
                    !item.signature.contains('{'),
                    "signature for '{}' leaks body: {}",
                    item.name,
                    item.signature
                );
            }
        }
    }

    #[test]
    fn line_numbers_are_one_based() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        for item in &items {
            assert!(
                item.start_line >= 1,
                "'{}' start_line should be >= 1, got {}",
                item.name,
                item.start_line
            );
            assert!(
                item.end_line >= item.start_line,
                "'{}' end_line {} < start_line {}",
                item.name,
                item.end_line,
                item.start_line
            );
        }
    }

    #[test]
    fn async_function_detected() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert!(f.metadata.is_async);
        let helper = find_by_name(&items, "internalHelper");
        assert!(!helper.metadata.is_async);
    }

    #[test]
    fn return_type_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert!(
            f.metadata.return_type.is_some(),
            "processItems should have return type"
        );
        let rt = f.metadata.return_type.as_deref().unwrap();
        assert!(
            rt.contains("Promise"),
            "return type should contain Promise: {rt}"
        );
    }

    #[test]
    fn type_parameters_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert!(
            f.metadata.type_parameters.is_some(),
            "processItems should have type params"
        );
        let tp = f.metadata.type_parameters.as_deref().unwrap();
        assert!(tp.contains('T'), "type params: {tp}");
    }

    #[test]
    fn tsx_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let root = SupportLang::Tsx.ast_grep(source);
        let items = extract(&root, SupportLang::Tsx).expect("extraction should succeed");
        let button = find_by_name(&items, "Button");
        assert_eq!(button.kind, SymbolKind::Function);
        assert_eq!(button.visibility, Visibility::Export);
    }

    #[test]
    fn tsx_default_export_function() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let root = SupportLang::Tsx.ast_grep(source);
        let items = extract(&root, SupportLang::Tsx).expect("extraction should succeed");
        let app = find_by_name(&items, "App");
        assert_eq!(app.kind, SymbolKind::Function);
        assert!(app.metadata.is_default_export);
    }

    // ── Namespace tests ────────────────────────────────────────────

    #[test]
    fn exported_namespace_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let ns = find_by_name(&items, "Validators");
        assert_eq!(ns.kind, SymbolKind::Module);
        assert_eq!(ns.visibility, Visibility::Export);
        assert!(ns.metadata.is_exported);
    }

    #[test]
    fn non_exported_namespace_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let ns = find_by_name(&items, "InternalUtils");
        assert_eq!(ns.kind, SymbolKind::Module);
        assert_eq!(ns.visibility, Visibility::Private);
    }

    #[test]
    fn namespace_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let ns = find_by_name(&items, "Validators");
        assert!(
            ns.doc_comment.contains("String validation"),
            "doc: {:?}",
            ns.doc_comment
        );
    }

    // ── Ambient declaration tests ──────────────────────────────────

    #[test]
    fn declare_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "fetchExternal");
        assert_eq!(f.kind, SymbolKind::Function);
    }

    #[test]
    fn declare_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "API_VERSION");
        assert_eq!(c.kind, SymbolKind::Const);
    }

    #[test]
    fn declare_class_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "ExternalLib");
        assert_eq!(c.kind, SymbolKind::Class);
    }

    #[test]
    fn declare_module_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "my-module");
        assert_eq!(m.kind, SymbolKind::Module);
        assert!(
            m.signature.contains("declare module"),
            "sig: {:?}",
            m.signature
        );
    }

    // ── Function signature (overload) tests ────────────────────────

    #[test]
    fn function_overload_signatures_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let greets: Vec<&ParsedItem> = items.iter().filter(|i| i.name == "greet").collect();
        assert!(
            greets.len() >= 3,
            "should find at least 3 greet items (2 overloads + 1 impl), found {}",
            greets.len()
        );
    }

    #[test]
    fn function_overload_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let greets: Vec<&ParsedItem> = items.iter().filter(|i| i.name == "greet").collect();
        let has_doc = greets
            .iter()
            .any(|g| g.doc_comment.contains("Greet a person"));
        assert!(has_doc, "at least one greet should have JSDoc");
    }

    // ── Variable declaration (var/let) tests ───────────────────────

    #[test]
    fn let_variable_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "counter");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.visibility, Visibility::Private);
    }

    #[test]
    fn var_variable_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "legacyFlag");
        assert_eq!(v.kind, SymbolKind::Const);
        assert_eq!(v.visibility, Visibility::Private);
    }

    // ── Enum with string values ────────────────────────────────────

    #[test]
    fn enum_with_string_values_variants_extracted() {
        let source = include_str!("../../tests/fixtures/sample.ts");
        let items = parse_and_extract(source);
        let e = find_by_name(&items, "Direction");
        assert_eq!(e.kind, SymbolKind::Enum);
        assert!(
            e.metadata.variants.contains(&"Up".to_string()),
            "variants: {:?}",
            e.metadata.variants
        );
        assert!(
            e.metadata.variants.contains(&"Down".to_string()),
            "variants: {:?}",
            e.metadata.variants
        );
        assert!(
            e.metadata.variants.contains(&"Left".to_string()),
            "variants: {:?}",
            e.metadata.variants
        );
        assert!(
            e.metadata.variants.contains(&"Right".to_string()),
            "variants: {:?}",
            e.metadata.variants
        );
    }
}
