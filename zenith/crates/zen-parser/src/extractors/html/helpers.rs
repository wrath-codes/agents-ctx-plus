use ast_grep_core::Node;

use crate::types::SymbolKind;

/// An HTML attribute: `(name, optional_value)`.
pub(super) type HtmlAttr = (String, Option<String>);

/// Extract tag name and attributes from an element's `start_tag`.
pub(super) fn extract_tag_info<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<(String, Vec<HtmlAttr>)> {
    for child in node.children() {
        if child.kind().as_ref() == "start_tag" {
            let tag_name = child
                .children()
                .find(|c| c.kind().as_ref() == "tag_name")?
                .text()
                .to_string();
            let attrs = extract_attrs_from_tag(&child);
            return Some((tag_name, attrs));
        }
        // Self-closing tags like <input /> or <meta>
        if child.kind().as_ref() == "self_closing_tag" {
            let tag_name = child
                .children()
                .find(|c| c.kind().as_ref() == "tag_name")?
                .text()
                .to_string();
            let attrs = extract_attrs_from_tag(&child);
            return Some((tag_name, attrs));
        }
    }
    None
}

/// Extract attributes from a `start_tag` node's children.
pub(super) fn extract_start_tag_attrs<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<HtmlAttr> {
    for child in node.children() {
        if child.kind().as_ref() == "start_tag" {
            return extract_attrs_from_tag(&child);
        }
    }
    Vec::new()
}

/// Extract all attributes from a tag node.
fn extract_attrs_from_tag<D: ast_grep_core::Doc>(tag_node: &Node<D>) -> Vec<HtmlAttr> {
    tag_node
        .children()
        .filter(|c| c.kind().as_ref() == "attribute")
        .filter_map(|attr| {
            let name = attr
                .children()
                .find(|c| c.kind().as_ref() == "attribute_name")?;
            let value = attr
                .children()
                .find(|c| c.kind().as_ref() == "quoted_attribute_value")
                .and_then(|qav| {
                    qav.children()
                        .find(|c| c.kind().as_ref() == "attribute_value")
                })
                .map(|v| v.text().to_string());
            Some((name.text().to_string(), value))
        })
        .collect()
}

pub(super) fn attr_value(attrs: &[HtmlAttr], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(n, _)| n == name)
        .and_then(|(_, v)| v.clone())
}

/// Tags that are significant enough to extract even without an `id`.
pub(super) fn is_significant_tag(tag: &str) -> bool {
    matches!(
        tag,
        // Semantic landmarks
        "nav" | "header" | "footer" | "main" | "aside"
            // Interactive / structural
            | "form" | "template" | "dialog" | "details"
            // Head resources
            | "meta" | "link"
            // Data
            | "table"
            // Embedded content
            | "iframe" | "object" | "embed"
            // Media
            | "video" | "audio" | "picture" | "canvas"
            // Form grouping / controls
            | "fieldset" | "select" | "output"
            // Web component slots
            | "slot"
    )
}

pub(super) fn classify_tag(tag: &str) -> SymbolKind {
    match tag {
        // Semantic landmarks -> Module
        "header" | "footer" | "main" | "nav" | "aside" | "section" | "article" => {
            SymbolKind::Module
        }
        // Head resources -> Const
        "meta" | "link" => SymbolKind::Const,
        // Embedded / media -> Static (external resources)
        "iframe" | "object" | "embed" | "video" | "audio" | "picture" | "canvas" => {
            SymbolKind::Static
        }
        // form, template, dialog, details, table, fieldset, select, output, slot, etc.
        _ => SymbolKind::Struct,
    }
}

pub(super) fn build_signature(tag: &str, attrs: &[HtmlAttr]) -> String {
    use std::fmt::Write;
    let mut sig = format!("<{tag}");
    for (name, value) in attrs {
        if let Some(val) = value {
            let _ = write!(sig, " {name}=\"{val}\"");
        } else {
            let _ = write!(sig, " {name}");
        }
    }
    sig.push('>');
    sig
}

#[allow(clippy::unnecessary_wraps)]
pub(super) fn extract_source_limited<D: ast_grep_core::Doc>(
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
            "{truncated}\n    <!-- ... ({} more lines) -->",
            lines.len() - max_lines
        ))
    }
}
