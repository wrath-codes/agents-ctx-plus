//! Shared extraction helpers used by all rich extractors.
//!
//! Functions for extracting signatures, doc comments, visibility,
//! attributes, and other metadata from AST nodes.

use ast_grep_core::Node;

use crate::types::Visibility;

/// Extract signature: everything before first `{` or `;`, whitespace-normalized.
///
/// Spike 0.21 finding: normalize whitespace (collapse newlines/runs to single space)
/// for deterministic signatures regardless of source formatting.
pub fn extract_signature<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let text = node.text().to_string();
    let brace = text.find('{');
    let semi = text.find(';');
    let end = match (brace, semi) {
        (Some(b), Some(s)) => b.min(s),
        (Some(b), None) => b,
        (None, Some(s)) => s,
        (None, None) => text.len(),
    };
    let sig = text[..end].trim();
    sig.replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract Python signature: definition line(s) before the body.
///
/// Python uses `:` to start the body, not `{`. The signature is
/// `def name(params) -> ReturnType` or `class Name(bases)`.
pub fn extract_signature_python<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    // Use the node's own fields to build a clean signature.
    let name = node
        .field("name")
        .map(|n| n.text().to_string())
        .unwrap_or_default();

    let kind = node.kind();
    let prefix = if kind.as_ref() == "class_definition" {
        "class"
    } else if node.text().starts_with("async ") {
        "async def"
    } else {
        "def"
    };

    let params = node
        .field("parameters")
        .map(|p| p.text().to_string());
    let superclasses = node
        .field("superclasses")
        .map(|s| s.text().to_string());
    let return_type = node
        .field("return_type")
        .map(|rt| format!(" -> {}", rt.text()));

    let mut sig = format!("{prefix} {name}");
    if let Some(p) = params {
        sig.push_str(&p);
    } else if let Some(s) = superclasses {
        sig.push_str(&s);
    }
    if let Some(rt) = return_type {
        sig.push_str(&rt);
    }
    sig
}

/// Extract full source up to `max_lines` lines.
#[allow(clippy::unnecessary_wraps)]
pub fn extract_source<D: ast_grep_core::Doc>(
    node: &Node<D>,
    max_lines: usize,
) -> Option<String> {
    let text = node.text().to_string();
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        Some(text)
    } else {
        let truncated: String = lines[..max_lines].join("\n");
        Some(format!(
            "{truncated}\n    // ... ({} more lines)",
            lines.len() - max_lines
        ))
    }
}

/// Extract Rust doc comments by walking backward through AST siblings.
///
/// Primary: walks `prev()` siblings collecting `///` and `//!` comments.
/// Fallback (spike 0.21): line-based scan above `start_pos().line()`.
pub fn extract_doc_comments_rust<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> String {
    // Primary: AST sibling walk
    let mut comments = Vec::new();
    let mut current = node.prev();
    while let Some(sibling) = current {
        let kind = sibling.kind();
        if kind.as_ref() == "line_comment" {
            let text = sibling.text().to_string();
            if text.starts_with("///") || text.starts_with("//!") {
                comments.push(
                    text.trim_start_matches("///")
                        .trim_start_matches("//!")
                        .trim()
                        .to_string(),
                );
            } else {
                break;
            }
        } else if kind.as_ref() == "attribute_item" {
            // Skip attributes, keep looking for docs
        } else {
            break;
        }
        current = sibling.prev();
    }
    if !comments.is_empty() {
        comments.reverse();
        return comments.join("\n");
    }

    // Fallback: line-based scan
    extract_doc_comments_rust_by_line(node, source)
}

/// Line-based fallback for Rust doc comment extraction.
fn extract_doc_comments_rust_by_line<D: ast_grep_core::Doc>(
    node: &Node<D>,
    source: &str,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = node.start_pos().line(); // zero-based
    if start_line == 0 || lines.is_empty() {
        return String::new();
    }
    let mut idx = start_line.saturating_sub(1);
    // Skip blank lines between doc and item
    while idx > 0 && lines.get(idx).is_some_and(|l| l.trim().is_empty()) {
        idx -= 1;
    }
    let mut docs = Vec::new();
    loop {
        let line = lines.get(idx).map_or("", |l| l.trim_start());
        if line.starts_with("///") {
            docs.push(line.trim_start_matches("///").trim().to_string());
        } else if line.starts_with("//!") {
            docs.push(line.trim_start_matches("//!").trim().to_string());
        } else {
            break;
        }
        if idx == 0 {
            break;
        }
        idx -= 1;
    }
    docs.reverse();
    docs.join("\n")
}

/// Extract `#[attr]` attributes from preceding siblings.
pub fn extract_attributes<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut current = node.prev();
    while let Some(sibling) = current {
        let kind = sibling.kind();
        if kind.as_ref() == "attribute_item" {
            let text = sibling.text().to_string();
            let inner = text
                .trim_start_matches("#[")
                .trim_end_matches(']')
                .to_string();
            attrs.push(inner);
        } else if kind.as_ref() == "line_comment" {
            // Skip comments between attributes
        } else {
            break;
        }
        current = sibling.prev();
    }
    attrs.reverse();
    attrs
}

/// Detect Rust visibility from a node's `visibility_modifier` child.
pub fn extract_visibility_rust<D: ast_grep_core::Doc>(node: &Node<D>) -> Visibility {
    for child in node.children() {
        if child.kind().as_ref() == "visibility_modifier" {
            let text = child.text().to_string();
            if text.contains("pub(crate)") {
                return Visibility::PublicCrate;
            } else if text.contains("pub(super)") {
                return Visibility::Private;
            } else if text.starts_with("pub") {
                return Visibility::Public;
            }
        }
    }
    Visibility::Private
}

/// Extract return type from a function node's `return_type` field.
pub fn extract_return_type<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("return_type")
        .map(|rt| rt.text().to_string().trim_start_matches("->").trim().to_string())
}

/// Extract generic/type parameters from a node.
pub fn extract_generics<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.field("type_parameters")
        .map(|tp| tp.text().to_string())
}

/// Extract where clause from a Rust node.
pub fn extract_where_clause<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    for child in node.children() {
        if child.kind().as_ref() == "where_clause" {
            return Some(child.text().to_string());
        }
    }
    None
}

/// Extract lifetime parameters from a generics string.
pub fn extract_lifetimes(generics: Option<&str>) -> Vec<String> {
    generics.map_or_else(Vec::new, |g| {
        let inner = g.trim_start_matches('<').trim_end_matches('>');
        let mut lifetimes = Vec::new();
        for part in inner.split(',') {
            let part = part.trim();
            if part.starts_with('\'') {
                let lt = part
                    .split(|c: char| !c.is_alphanumeric() && c != '\'')
                    .next()
                    .unwrap_or(part);
                lifetimes.push(lt.to_string());
            }
        }
        lifetimes
    })
}

/// Check if a return type contains `Result`.
pub fn returns_result(return_type: Option<&str>) -> bool {
    return_type.is_some_and(|rt| rt.contains("Result"))
}

/// Check if a type name indicates an error type by naming convention.
pub fn is_error_type_by_name(name: &str) -> bool {
    name.ends_with("Error") || name.ends_with("Err")
}

/// Check if an item has `PyO3` attributes.
pub fn is_pyo3(attrs: &[String]) -> bool {
    attrs.iter().any(|a| {
        a.starts_with("pyfunction")
            || a.starts_with("pyclass")
            || a.starts_with("pymethods")
    })
}

/// Detect `async`/`unsafe` modifiers on a function node.
///
/// Spike 0.8 finding (b): `async`/`unsafe` appear as children of
/// `function_modifiers` node, not as direct children of the function.
pub fn detect_modifiers<D: ast_grep_core::Doc>(node: &Node<D>) -> (bool, bool) {
    let mut is_async = false;
    let mut is_unsafe = false;
    for child in node.children() {
        let kind = child.kind();
        let k = kind.as_ref();
        if k == "function_modifiers" {
            let text = child.text().to_string();
            is_async = text.contains("async");
            is_unsafe = text.contains("unsafe");
            break;
        }
        if k == "async" {
            is_async = true;
        }
        if k == "unsafe" {
            is_unsafe = true;
        }
    }
    is_async = is_async || node.text().starts_with("async ");
    is_unsafe = is_unsafe || node.text().starts_with("unsafe ");
    (is_async, is_unsafe)
}

/// Parse Rust doc sections (`# Errors`, `# Panics`, `# Safety`, `# Examples`).
pub fn parse_rust_doc_sections(doc: &str) -> crate::types::DocSections {
    let mut sections = crate::types::DocSections::default();
    let mut current_section: Option<&str> = None;
    let mut current_content = String::new();

    for line in doc.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            flush_section(&mut sections, current_section, &current_content);
            current_section = Some(heading.trim());
            current_content.clear();
        } else {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(trimmed);
        }
    }
    flush_section(&mut sections, current_section, &current_content);
    sections
}

fn flush_section(
    sections: &mut crate::types::DocSections,
    heading: Option<&str>,
    content: &str,
) {
    let content = content.trim();
    if content.is_empty() {
        return;
    }
    match heading {
        Some("Errors") => sections.errors = Some(content.to_string()),
        Some("Panics") => sections.panics = Some(content.to_string()),
        Some("Safety") => sections.safety = Some(content.to_string()),
        Some("Examples") => sections.examples = Some(content.to_string()),
        Some("Returns") => sections.returns = Some(content.to_string()),
        _ => {}
    }
}

/// Extract parameters from a Rust function node.
pub fn extract_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(params_node) = node.field("parameters") else {
        return Vec::new();
    };
    params_node
        .children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() == "parameter" || k.as_ref() == "self_parameter"
        })
        .map(|c| c.text().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast_grep_language::{LanguageExt, SupportLang};

    #[test]
    fn signature_excludes_body() {
        let source = "fn hello(x: i32) -> bool { true }";
        let root = SupportLang::Rust.ast_grep(source);
        let func = root
            .root()
            .find(ast_grep_core::matcher::KindMatcher::new(
                "function_item",
                SupportLang::Rust,
            ))
            .expect("should find function");
        let sig = extract_signature(&func);
        assert!(!sig.contains('{'));
        assert!(sig.contains("fn hello"));
    }

    #[test]
    fn signature_normalizes_whitespace() {
        let source = "fn hello(\n    x: i32,\n    y: i32,\n) -> bool {\n    true\n}";
        let root = SupportLang::Rust.ast_grep(source);
        let func = root
            .root()
            .find(ast_grep_core::matcher::KindMatcher::new(
                "function_item",
                SupportLang::Rust,
            ))
            .expect("should find function");
        let sig = extract_signature(&func);
        assert!(!sig.contains('\n'));
    }

    #[test]
    fn source_truncates_long_functions() {
        let lines: Vec<String> = (0..60).map(|i| format!("    let x{i} = {i};")).collect();
        let body = lines.join("\n");
        let source = format!("fn big() {{\n{body}\n}}");
        let root = SupportLang::Rust.ast_grep(&source);
        let func = root
            .root()
            .find(ast_grep_core::matcher::KindMatcher::new(
                "function_item",
                SupportLang::Rust,
            ))
            .expect("should find function");
        let src = extract_source(&func, 50);
        assert!(src.is_some());
        assert!(src.unwrap().contains("more lines"));
    }

    #[test]
    fn visibility_detection() {
        let source = "pub fn a() {} fn b() {} pub(crate) fn c() {}";
        let root = SupportLang::Rust.ast_grep(source);
        let funcs: Vec<_> = root
            .root()
            .find_all(ast_grep_core::matcher::KindMatcher::new(
                "function_item",
                SupportLang::Rust,
            ))
            .collect();
        assert_eq!(funcs.len(), 3);

        let mut vis_map = std::collections::HashMap::new();
        for f in &funcs {
            let name = f
                .field("name")
                .map(|n| n.text().to_string())
                .unwrap_or_default();
            vis_map.insert(name, extract_visibility_rust(f));
        }
        assert_eq!(vis_map["a"], Visibility::Public);
        assert_eq!(vis_map["b"], Visibility::Private);
        assert_eq!(vis_map["c"], Visibility::PublicCrate);
    }

    #[test]
    fn detect_async_modifier() {
        let source = "pub async fn fetch() {}";
        let root = SupportLang::Rust.ast_grep(source);
        let func = root
            .root()
            .find(ast_grep_core::matcher::KindMatcher::new(
                "function_item",
                SupportLang::Rust,
            ))
            .expect("should find function");
        let (is_async, is_unsafe) = detect_modifiers(&func);
        assert!(is_async);
        assert!(!is_unsafe);
    }

    #[test]
    fn detect_unsafe_modifier() {
        let source = "unsafe fn danger() {}";
        let root = SupportLang::Rust.ast_grep(source);
        let func = root
            .root()
            .find(ast_grep_core::matcher::KindMatcher::new(
                "function_item",
                SupportLang::Rust,
            ))
            .expect("should find function");
        let (is_async, is_unsafe) = detect_modifiers(&func);
        assert!(!is_async);
        assert!(is_unsafe);
    }

    #[test]
    fn returns_result_detection() {
        assert!(returns_result(Some("Result<(), Error>")));
        assert!(!returns_result(Some("String")));
        assert!(!returns_result(None));
    }

    #[test]
    fn error_type_by_name_detection() {
        assert!(is_error_type_by_name("MyError"));
        assert!(is_error_type_by_name("ParseErr"));
        assert!(!is_error_type_by_name("Config"));
    }

    #[test]
    fn lifetime_extraction() {
        let lts = extract_lifetimes(Some("<'a, 'b, T: Clone>"));
        assert_eq!(lts, vec!["'a", "'b"]);
    }

    #[test]
    fn lifetime_extraction_none() {
        let lts = extract_lifetimes(None);
        assert!(lts.is_empty());
    }

    #[test]
    fn pyo3_detection() {
        assert!(is_pyo3(&["pyfunction".to_string()]));
        assert!(is_pyo3(&["pyclass".to_string()]));
        assert!(!is_pyo3(&["derive(Debug)".to_string()]));
    }

    #[test]
    fn doc_section_parsing() {
        let doc = "Does something.\n# Errors\nReturns Err on failure.\n# Panics\nNever panics.";
        let sections = parse_rust_doc_sections(doc);
        assert_eq!(sections.errors, Some("Returns Err on failure.".to_string()));
        assert_eq!(sections.panics, Some("Never panics.".to_string()));
    }
}
