//! JavaScript rich extractor.
//!
//! Handles plain JavaScript (ES2015+) including generator functions,
//! classes with getters/setters/static methods, arrow functions,
//! `export` statements, and `JSDoc` comment extraction.

use ast_grep_core::Node;
use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::ops::Any;
use ast_grep_language::SupportLang;

use super::helpers;
use crate::types::{DocSections, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

const JS_TOP_KINDS: &[&str] = &[
    "export_statement",
    "function_declaration",
    "generator_function_declaration",
    "class_declaration",
    "lexical_declaration",
    "variable_declaration",
];

/// Extract all API symbols from a JavaScript source file.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let matchers: Vec<KindMatcher> = JS_TOP_KINDS
        .iter()
        .map(|k| KindMatcher::new(k, SupportLang::JavaScript))
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
            "generator_function_declaration" => {
                if let Some(item) = process_generator_function(&node, &node, false) {
                    items.push(item);
                }
            }
            "class_declaration" => {
                if let Some(item) = process_class(&node, &node, false, false) {
                    items.push(item);
                }
            }
            "lexical_declaration" => {
                items.extend(process_lexical_declaration(&node, &node, false));
            }
            "variable_declaration" => {
                items.extend(process_variable_declaration(&node, &node, false));
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
            "generator_function_declaration" => {
                if let Some(item) = process_generator_function(&child, export_node, true) {
                    items.push(item);
                }
            }
            "class_declaration" => {
                if let Some(item) = process_class(&child, export_node, true, is_default) {
                    items.push(item);
                }
            }
            "lexical_declaration" => {
                items.extend(process_lexical_declaration(&child, export_node, true));
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

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            is_async,
            is_exported,
            is_default_export: is_default,
            parameters: extract_js_parameters(node),
            doc_sections,
            ..Default::default()
        },
    })
}

// ── generator_function_declaration ─────────────────────────────────

fn process_generator_function<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);
    let is_async = node.children().any(|c| c.kind().as_ref() == "async");

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    Some(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            is_async,
            is_exported,
            is_generator: true,
            parameters: extract_js_parameters(node),
            doc_sections,
            ..Default::default()
        },
    })
}

// ── class_declaration ──────────────────────────────────────────────

fn process_class<D: ast_grep_core::Doc>(
    node: &Node<D>,
    jsdoc_anchor: &Node<D>,
    is_exported: bool,
    is_default: bool,
) -> Option<ParsedItem> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let jsdoc = extract_jsdoc_before(jsdoc_anchor);
    let doc_sections = parse_jsdoc_sections(&jsdoc);

    let extends = extract_class_heritage(node);
    let methods = extract_class_methods(node);

    let is_error_type =
        helpers::is_error_type_by_name(&name) || extends.iter().any(|e| e == "Error");

    let visibility = if is_exported {
        Visibility::Export
    } else {
        Visibility::Private
    };

    Some(ParsedItem {
        kind: SymbolKind::Class,
        name,
        signature: helpers::extract_signature(node),
        source: helpers::extract_source(node, 50),
        doc_comment: jsdoc,
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata: SymbolMetadata {
            is_exported,
            is_default_export: is_default,
            base_classes: extends,
            methods,
            is_error_type,
            doc_sections,
            ..Default::default()
        },
    })
}

fn extract_class_heritage<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    for child in node.children() {
        if child.kind().as_ref() == "class_heritage" {
            // JS: class_heritage → extends + identifier (no extends_clause wrapper)
            return child
                .children()
                .filter(|c| {
                    let k = c.kind();
                    k.as_ref() != "extends" && k.as_ref() != ","
                })
                .map(|c| c.text().to_string())
                .collect();
        }
    }
    Vec::new()
}

fn extract_class_methods<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut methods = Vec::new();
    let Some(body) = node.field("body") else {
        return methods;
    };

    for child in body.children() {
        if child.kind().as_ref() == "method_definition"
            && let Some(name) = child.field("name").map(|n| n.text().to_string())
        {
            methods.push(name);
        }
    }
    methods
}

// ── lexical_declaration (const/let with arrow functions or values) ─

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

// ── variable_declaration (var) ─────────────────────────────────────

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
        let params = extract_js_parameters(&arrow);

        Some(ParsedItem {
            kind: SymbolKind::Function,
            name,
            signature: helpers::extract_signature(declaration),
            source: helpers::extract_source(declaration, 50),
            doc_comment: jsdoc,
            start_line: declaration.start_pos().line() as u32 + 1,
            end_line: declaration.end_pos().line() as u32 + 1,
            visibility,
            metadata: SymbolMetadata {
                is_async,
                is_exported,
                parameters: params,
                doc_sections,
                ..Default::default()
            },
        })
    } else {
        Some(ParsedItem {
            kind: SymbolKind::Const,
            name,
            signature: helpers::extract_signature(declaration),
            source: helpers::extract_source(declaration, 50),
            doc_comment: jsdoc,
            start_line: declaration.start_pos().line() as u32 + 1,
            end_line: declaration.end_pos().line() as u32 + 1,
            visibility,
            metadata: SymbolMetadata {
                is_exported,
                doc_sections,
                ..Default::default()
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
            // Handle @param {type} name desc or @param name desc
            let rest = if rest.starts_with('{') {
                // Skip {type} prefix
                rest.split_once('}')
                    .map_or(rest, |(_, after)| after.trim_start())
            } else {
                rest
            };
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
        } else if let Some(rest) = trimmed.strip_prefix("@yields ") {
            sections.yields = Some(rest.trim().to_string());
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

// ── JS-specific helpers ────────────────────────────────────────────

fn extract_js_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(params) = node.field("parameters") else {
        return Vec::new();
    };
    params
        .children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "identifier"
                || k.as_ref() == "assignment_pattern"
                || k.as_ref() == "rest_pattern"
                || k.as_ref() == "object_pattern"
                || k.as_ref() == "array_pattern"
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
        let root = SupportLang::JavaScript.ast_grep(source);
        extract(&root).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("should find item named '{name}'"))
    }

    // ── Regular function tests ─────────────────────────────────────

    #[test]
    fn function_with_jsdoc_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "sum");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Private);
        assert!(!f.metadata.is_async);
        assert!(!f.metadata.is_exported);
    }

    #[test]
    fn function_jsdoc_content() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "sum");
        assert!(
            f.doc_comment.contains("Calculate the sum"),
            "doc: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn function_jsdoc_params_parsed() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "sum");
        assert!(
            f.metadata.doc_sections.args.contains_key("numbers"),
            "args: {:?}",
            f.metadata.doc_sections.args
        );
    }

    #[test]
    fn function_jsdoc_returns_parsed() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "sum");
        assert!(
            f.metadata.doc_sections.returns.is_some(),
            "should have @returns"
        );
    }

    #[test]
    fn non_documented_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "internalHelper");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Private);
        assert!(f.doc_comment.is_empty());
    }

    #[test]
    fn function_parameters_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "sum");
        assert!(
            f.metadata.parameters.contains(&"numbers".to_string()),
            "params: {:?}",
            f.metadata.parameters
        );
    }

    // ── Async function tests ───────────────────────────────────────

    #[test]
    fn async_function_detected() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "fetchData");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(f.metadata.is_async);
    }

    // ── Generator function tests ───────────────────────────────────

    #[test]
    fn generator_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "generateNumbers");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(f.metadata.is_generator);
        assert!(!f.metadata.is_async);
    }

    #[test]
    fn generator_function_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "generateNumbers");
        assert!(
            f.doc_comment.contains("Generate sequential numbers"),
            "doc: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn generator_jsdoc_yields_parsed() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "generateNumbers");
        assert!(
            f.metadata.doc_sections.yields.is_some(),
            "should have @yields"
        );
    }

    #[test]
    fn async_generator_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "asyncStream");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(f.metadata.is_generator);
        assert!(f.metadata.is_async);
    }

    // ── Class tests ────────────────────────────────────────────────

    #[test]
    fn class_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Animal");
        assert_eq!(c.kind, SymbolKind::Class);
        assert_eq!(c.visibility, Visibility::Private);
    }

    #[test]
    fn class_methods_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Animal");
        assert!(
            c.metadata.methods.contains(&"constructor".to_string()),
            "methods: {:?}",
            c.metadata.methods
        );
        assert!(
            c.metadata.methods.contains(&"speak".to_string()),
            "methods: {:?}",
            c.metadata.methods
        );
        assert!(
            c.metadata.methods.contains(&"create".to_string()),
            "methods: {:?}",
            c.metadata.methods
        );
    }

    #[test]
    fn class_getter_setter_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Animal");
        // Getters/setters are method_definition nodes, captured by name
        let display_count = c
            .metadata
            .methods
            .iter()
            .filter(|m| *m == "displayName")
            .count();
        assert!(
            display_count >= 1,
            "should have at least one displayName method, methods: {:?}",
            c.metadata.methods
        );
    }

    #[test]
    fn error_class_detected() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "ValidationError");
        assert_eq!(c.kind, SymbolKind::Class);
        assert!(c.metadata.is_error_type);
        assert!(
            c.metadata.base_classes.contains(&"Error".to_string()),
            "base_classes: {:?}",
            c.metadata.base_classes
        );
    }

    // ── Arrow function tests ───────────────────────────────────────

    #[test]
    fn arrow_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "multiply");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Private);
        assert!(!f.metadata.is_async);
    }

    #[test]
    fn arrow_function_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "multiply");
        assert!(
            f.doc_comment.contains("Multiply two numbers"),
            "doc: {:?}",
            f.doc_comment
        );
    }

    #[test]
    fn async_arrow_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "asyncTransform");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(f.metadata.is_async);
    }

    // ── Constant/variable tests ────────────────────────────────────

    #[test]
    fn const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "MAX_RETRIES");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.visibility, Visibility::Private);
    }

    #[test]
    fn let_variable_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "mutableCounter");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.visibility, Visibility::Private);
    }

    #[test]
    fn var_variable_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "legacyFlag");
        assert_eq!(v.kind, SymbolKind::Const);
        assert_eq!(v.visibility, Visibility::Private);
    }

    // ── Export tests ───────────────────────────────────────────────

    #[test]
    fn exported_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "formatDate");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Export);
        assert!(f.metadata.is_exported);
    }

    #[test]
    fn exported_class_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "EventBus");
        assert_eq!(c.kind, SymbolKind::Class);
        assert_eq!(c.visibility, Visibility::Export);
        assert!(c.metadata.is_exported);
    }

    #[test]
    fn exported_class_methods_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "EventBus");
        assert!(
            c.metadata.methods.contains(&"emit".to_string()),
            "methods: {:?}",
            c.metadata.methods
        );
    }

    #[test]
    fn exported_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "VERSION");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.visibility, Visibility::Export);
    }

    #[test]
    fn exported_arrow_function_extracted() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "processItems");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Export);
        assert!(f.metadata.is_async);
    }

    #[test]
    fn default_export_function() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "main");
        assert_eq!(f.kind, SymbolKind::Function);
        assert_eq!(f.visibility, Visibility::Export);
        assert!(f.metadata.is_default_export);
        assert!(f.metadata.is_exported);
    }

    // ── Signature tests ────────────────────────────────────────────

    #[test]
    fn signature_no_body_leak() {
        let source = include_str!("../../tests/fixtures/sample.js");
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
        let source = include_str!("../../tests/fixtures/sample.js");
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

    // ── Generator signature test ───────────────────────────────────

    #[test]
    fn generator_function_signature_has_star() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "generateNumbers");
        assert!(
            f.signature.contains("function*") || f.signature.contains("function *"),
            "generator sig should contain '*': {}",
            f.signature
        );
    }

    #[test]
    fn generator_function_parameters() {
        let source = include_str!("../../tests/fixtures/sample.js");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "generateNumbers");
        assert!(
            f.metadata.parameters.contains(&"max".to_string()),
            "params: {:?}",
            f.metadata.parameters
        );
    }
}
