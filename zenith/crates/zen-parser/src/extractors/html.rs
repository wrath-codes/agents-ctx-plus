//! HTML rich extractor.
//!
//! Extracts structurally significant elements from HTML documents:
//! custom elements (web components), elements with `id` attributes,
//! `<template>` elements, `<form>` elements, `<script>`/`<link>` resource
//! references, and semantic landmarks.

use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

/// An HTML attribute: `(name, optional_value)`.
type HtmlAttr = (String, Option<String>);

/// Extract all significant elements from an HTML document.
///
/// Walks the entire document tree collecting:
/// - Custom elements (tag names containing `-`)
/// - Elements with `id` attributes
/// - `<template>`, `<form>`, `<dialog>`, `<details>` elements
/// - `<script>` and `<link>` resource references
/// - `<meta>` tags with `name` attribute
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let root_node = root.root();
    collect_elements(&root_node, &mut items);
    Ok(items)
}

fn collect_elements<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let kind = node.kind();
    match kind.as_ref() {
        "element" => process_element(node, items),
        "script_element" => process_script_element(node, items),
        "style_element" => process_style_element(node, items),
        _ => {}
    }

    // Recurse into children without holding borrows across iterations
    let children: Vec<_> = node.children().collect();
    for child in &children {
        collect_elements(child, items);
    }
}

// ── element processing ─────────────────────────────────────────────

fn process_element<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let Some((tag_name, attrs)) = extract_tag_info(node) else {
        return;
    };

    let element_id = attr_value(&attrs, "id");
    let class_names = attr_value(&attrs, "class")
        .map(|c| c.split_whitespace().map(String::from).collect::<Vec<_>>())
        .unwrap_or_default();
    let is_custom = tag_name.contains('-');
    let has_end_tag = node.children().any(|c| c.kind().as_ref() == "end_tag");
    let is_self_closing = !has_end_tag;

    let should_extract = is_custom || element_id.is_some() || is_significant_tag(&tag_name);

    if !should_extract {
        return;
    }

    let symbol_kind = if is_custom {
        SymbolKind::Component
    } else {
        classify_tag(&tag_name)
    };

    let name = if is_custom {
        tag_name.clone()
    } else if let Some(ref id) = element_id {
        id.clone()
    } else {
        tag_name.clone()
    };

    let signature = build_signature(&tag_name, &attrs);

    items.push(ParsedItem {
        kind: symbol_kind,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            tag_name: Some(tag_name),
            element_id,
            class_names,
            html_attributes: attrs,
            is_custom_element: is_custom,
            is_self_closing,
            ..Default::default()
        },
    });
}

// ── script_element processing ──────────────────────────────────────

fn process_script_element<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let attrs = extract_start_tag_attrs(node);
    let src = attr_value(&attrs, "src");
    let script_type = attr_value(&attrs, "type");

    let name = match src {
        Some(ref s) => s.clone(),
        None if script_type.as_deref() == Some("module") => "inline-module".to_string(),
        None => "inline-script".to_string(),
    };

    let signature = build_signature("script", &attrs);

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 10),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            tag_name: Some("script".to_string()),
            html_attributes: attrs,
            ..Default::default()
        },
    });
}

// ── style_element processing ───────────────────────────────────────

fn process_style_element<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let attrs = extract_start_tag_attrs(node);
    let signature = build_signature("style", &attrs);

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name: "inline-style".to_string(),
        signature,
        source: extract_source_limited(node, 10),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata: SymbolMetadata {
            tag_name: Some("style".to_string()),
            html_attributes: attrs,
            ..Default::default()
        },
    });
}

// ── Helper functions ───────────────────────────────────────────────

/// Extract tag name and attributes from an element's `start_tag`.
fn extract_tag_info<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<(String, Vec<HtmlAttr>)> {
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
fn extract_start_tag_attrs<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<HtmlAttr> {
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

fn attr_value(attrs: &[HtmlAttr], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(n, _)| n == name)
        .and_then(|(_, v)| v.clone())
}

/// Tags that are significant enough to extract even without an `id`.
fn is_significant_tag(tag: &str) -> bool {
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

fn classify_tag(tag: &str) -> SymbolKind {
    match tag {
        // Semantic landmarks → Module
        "header" | "footer" | "main" | "nav" | "aside" | "section" | "article" => {
            SymbolKind::Module
        }
        // Head resources → Const
        "meta" | "link" => SymbolKind::Const,
        // Embedded / media → Static (external resources)
        "iframe" | "object" | "embed" | "video" | "audio" | "picture" | "canvas" => {
            SymbolKind::Static
        }
        // form, template, dialog, details, table, fieldset, select, output, slot, etc.
        _ => SymbolKind::Struct,
    }
}

fn build_signature(tag: &str, attrs: &[HtmlAttr]) -> String {
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
fn extract_source_limited<D: ast_grep_core::Doc>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use ast_grep_language::LanguageExt;
    use pretty_assertions::assert_eq;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::Html.ast_grep(source);
        extract(&root).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("should find item named '{name}'"))
    }

    fn find_all_by_tag<'a>(items: &'a [ParsedItem], tag: &str) -> Vec<&'a ParsedItem> {
        items
            .iter()
            .filter(|i| i.metadata.tag_name.as_deref() == Some(tag))
            .collect()
    }

    // ── Custom element tests ───────────────────────────────────────

    #[test]
    fn custom_element_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "my-component");
        assert_eq!(c.kind, SymbolKind::Component);
        assert!(c.metadata.is_custom_element);
    }

    #[test]
    fn custom_element_attributes() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "my-component");
        assert!(
            c.metadata
                .html_attributes
                .iter()
                .any(|(n, _)| n == "data-id"),
            "attrs: {:?}",
            c.metadata.html_attributes
        );
    }

    #[test]
    fn custom_element_x_modal() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "x-modal");
        assert_eq!(c.kind, SymbolKind::Component);
        assert!(c.metadata.is_custom_element);
    }

    #[test]
    fn custom_element_app_header() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "app-header");
        assert_eq!(c.kind, SymbolKind::Component);
        assert!(c.metadata.is_custom_element);
    }

    // ── Elements with id tests ─────────────────────────────────────

    #[test]
    fn element_with_id_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let h = find_by_name(&items, "main-header");
        assert_eq!(h.metadata.tag_name.as_deref(), Some("header"));
        assert_eq!(h.metadata.element_id.as_deref(), Some("main-header"));
    }

    #[test]
    fn element_classes_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let h = find_by_name(&items, "main-header");
        assert!(
            h.metadata.class_names.contains(&"site-header".to_string()),
            "classes: {:?}",
            h.metadata.class_names
        );
        assert!(
            h.metadata.class_names.contains(&"sticky".to_string()),
            "classes: {:?}",
            h.metadata.class_names
        );
    }

    #[test]
    fn nav_with_id_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let nav = find_by_name(&items, "main-nav");
        assert_eq!(nav.metadata.tag_name.as_deref(), Some("nav"));
    }

    #[test]
    fn content_main_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "content");
        assert_eq!(m.metadata.tag_name.as_deref(), Some("main"));
    }

    #[test]
    fn section_with_id_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "hero");
        assert_eq!(s.metadata.tag_name.as_deref(), Some("section"));
    }

    #[test]
    fn article_with_id_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let a = find_by_name(&items, "feature-1");
        assert_eq!(a.metadata.tag_name.as_deref(), Some("article"));
    }

    #[test]
    fn article_data_attribute() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let a = find_by_name(&items, "feature-1");
        assert!(
            a.metadata
                .html_attributes
                .iter()
                .any(|(n, v)| n == "data-category" && v.as_deref() == Some("core")),
            "attrs: {:?}",
            a.metadata.html_attributes
        );
    }

    // ── Form tests ─────────────────────────────────────────────────

    #[test]
    fn form_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "contact-form");
        assert_eq!(f.kind, SymbolKind::Struct);
        assert_eq!(f.metadata.tag_name.as_deref(), Some("form"));
    }

    #[test]
    fn form_action_in_attributes() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "contact-form");
        assert!(
            f.metadata
                .html_attributes
                .iter()
                .any(|(n, v)| n == "action" && v.as_deref() == Some("/api/contact")),
            "attrs: {:?}",
            f.metadata.html_attributes
        );
    }

    #[test]
    fn form_method_in_attributes() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "contact-form");
        assert!(
            f.metadata
                .html_attributes
                .iter()
                .any(|(n, v)| n == "method" && v.as_deref() == Some("POST")),
            "attrs: {:?}",
            f.metadata.html_attributes
        );
    }

    #[test]
    fn form_input_with_id_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let input = find_by_name(&items, "name-input");
        assert_eq!(input.metadata.tag_name.as_deref(), Some("input"));
    }

    // ── Template tests ─────────────────────────────────────────────

    #[test]
    fn template_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "card-template");
        assert_eq!(t.kind, SymbolKind::Struct);
        assert_eq!(t.metadata.tag_name.as_deref(), Some("template"));
    }

    // ── Dialog tests ───────────────────────────────────────────────

    #[test]
    fn dialog_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let d = find_by_name(&items, "confirm-dialog");
        assert_eq!(d.kind, SymbolKind::Struct);
        assert_eq!(d.metadata.tag_name.as_deref(), Some("dialog"));
    }

    #[test]
    fn dialog_classes() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let d = find_by_name(&items, "confirm-dialog");
        assert!(
            d.metadata.class_names.contains(&"modal".to_string()),
            "classes: {:?}",
            d.metadata.class_names
        );
    }

    // ── Details tests ──────────────────────────────────────────────

    #[test]
    fn details_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let d = find_by_name(&items, "faq-section");
        assert_eq!(d.kind, SymbolKind::Struct);
        assert_eq!(d.metadata.tag_name.as_deref(), Some("details"));
    }

    // ── Script tests ───────────────────────────────────────────────

    #[test]
    fn script_with_src_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "vendor.js");
        assert_eq!(s.kind, SymbolKind::Module);
        assert_eq!(s.metadata.tag_name.as_deref(), Some("script"));
    }

    #[test]
    fn script_module_with_src_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "app.js");
        assert_eq!(s.kind, SymbolKind::Module);
    }

    #[test]
    fn inline_module_script_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "inline-module");
        assert_eq!(s.kind, SymbolKind::Module);
    }

    // ── Link tests ─────────────────────────────────────────────────

    #[test]
    fn link_elements_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let links = find_all_by_tag(&items, "link");
        assert!(
            links.len() >= 2,
            "should find at least 2 link elements, found {}",
            links.len()
        );
    }

    // ── Meta tests ─────────────────────────────────────────────────

    #[test]
    fn meta_tags_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let metas = find_all_by_tag(&items, "meta");
        assert!(!metas.is_empty(), "should find meta elements");
    }

    // ── Semantic landmark tests ────────────────────────────────────

    #[test]
    fn footer_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "main-footer");
        assert_eq!(f.metadata.tag_name.as_deref(), Some("footer"));
    }

    #[test]
    fn aside_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let a = find_by_name(&items, "sidebar");
        assert_eq!(a.metadata.tag_name.as_deref(), Some("aside"));
    }

    // ── Signature tests ────────────────────────────────────────────

    #[test]
    fn signature_includes_tag_and_attrs() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "contact-form");
        assert!(f.signature.starts_with("<form"), "sig: {}", f.signature);
        assert!(f.signature.contains("action="), "sig: {}", f.signature);
    }

    #[test]
    fn custom_element_signature() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "my-component");
        assert!(
            c.signature.starts_with("<my-component"),
            "sig: {}",
            c.signature
        );
    }

    // ── Line number tests ──────────────────────────────────────────

    #[test]
    fn line_numbers_are_one_based() {
        let source = include_str!("../../tests/fixtures/sample.html");
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

    // ── Style element test ─────────────────────────────────────────

    #[test]
    fn inline_style_extraction() {
        let source = "<style>.card { display: flex; }</style>";
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "inline-style");
        assert_eq!(s.kind, SymbolKind::Module);
        assert_eq!(s.metadata.tag_name.as_deref(), Some("style"));
    }

    // ── Self-closing test ──────────────────────────────────────────

    #[test]
    fn self_closing_element_detected() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        // meta tags are self-closing
        let metas = find_all_by_tag(&items, "meta");
        for meta in &metas {
            assert!(
                meta.metadata.is_self_closing,
                "meta should be self-closing: {}",
                meta.name
            );
        }
    }

    // ── Table tests ────────────────────────────────────────────────

    #[test]
    fn table_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "data-table");
        assert_eq!(t.kind, SymbolKind::Struct);
        assert_eq!(t.metadata.tag_name.as_deref(), Some("table"));
    }

    #[test]
    fn table_classes() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "data-table");
        assert!(
            t.metadata.class_names.contains(&"striped".to_string()),
            "classes: {:?}",
            t.metadata.class_names
        );
    }

    // ── Iframe tests ───────────────────────────────────────────────

    #[test]
    fn iframe_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "embed-frame");
        assert_eq!(i.kind, SymbolKind::Static);
        assert_eq!(i.metadata.tag_name.as_deref(), Some("iframe"));
    }

    #[test]
    fn iframe_src_in_attributes() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "embed-frame");
        assert!(
            i.metadata
                .html_attributes
                .iter()
                .any(|(n, v)| n == "src" && v.as_deref() == Some("https://example.com")),
            "attrs: {:?}",
            i.metadata.html_attributes
        );
    }

    // ── Object / Embed tests ───────────────────────────────────────

    #[test]
    fn object_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let o = find_by_name(&items, "flash-obj");
        assert_eq!(o.kind, SymbolKind::Static);
        assert_eq!(o.metadata.tag_name.as_deref(), Some("object"));
    }

    #[test]
    fn embed_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let e = find_by_name(&items, "pdf-embed");
        assert_eq!(e.kind, SymbolKind::Static);
        assert_eq!(e.metadata.tag_name.as_deref(), Some("embed"));
    }

    // ── Video / Audio tests ────────────────────────────────────────

    #[test]
    fn video_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "intro-video");
        assert_eq!(v.kind, SymbolKind::Static);
        assert_eq!(v.metadata.tag_name.as_deref(), Some("video"));
    }

    #[test]
    fn video_attributes() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let v = find_by_name(&items, "intro-video");
        assert!(
            v.metadata
                .html_attributes
                .iter()
                .any(|(n, _)| n == "controls"),
            "attrs: {:?}",
            v.metadata.html_attributes
        );
        assert!(
            v.metadata
                .html_attributes
                .iter()
                .any(|(n, _)| n == "autoplay"),
            "attrs: {:?}",
            v.metadata.html_attributes
        );
    }

    #[test]
    fn audio_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let a = find_by_name(&items, "bg-music");
        assert_eq!(a.kind, SymbolKind::Static);
        assert_eq!(a.metadata.tag_name.as_deref(), Some("audio"));
    }

    // ── Picture / Canvas tests ─────────────────────────────────────

    #[test]
    fn picture_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "hero-picture");
        assert_eq!(p.kind, SymbolKind::Static);
        assert_eq!(p.metadata.tag_name.as_deref(), Some("picture"));
    }

    #[test]
    fn canvas_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "game-canvas");
        assert_eq!(c.kind, SymbolKind::Static);
        assert_eq!(c.metadata.tag_name.as_deref(), Some("canvas"));
    }

    #[test]
    fn canvas_dimensions() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "game-canvas");
        assert!(
            c.metadata
                .html_attributes
                .iter()
                .any(|(n, v)| n == "width" && v.as_deref() == Some("800")),
            "attrs: {:?}",
            c.metadata.html_attributes
        );
    }

    // ── Fieldset / Select / Output tests ───────────────────────────

    #[test]
    fn fieldset_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "preferences");
        assert_eq!(f.kind, SymbolKind::Struct);
        assert_eq!(f.metadata.tag_name.as_deref(), Some("fieldset"));
    }

    #[test]
    fn fieldset_classes() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "preferences");
        assert!(
            f.metadata.class_names.contains(&"pref-group".to_string()),
            "classes: {:?}",
            f.metadata.class_names
        );
    }

    #[test]
    fn select_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "country-select");
        assert_eq!(s.kind, SymbolKind::Struct);
        assert_eq!(s.metadata.tag_name.as_deref(), Some("select"));
    }

    #[test]
    fn output_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let o = find_by_name(&items, "calc-result");
        assert_eq!(o.kind, SymbolKind::Struct);
        assert_eq!(o.metadata.tag_name.as_deref(), Some("output"));
    }

    // ── Slot test ──────────────────────────────────────────────────

    #[test]
    fn slot_extracted() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let slots = find_all_by_tag(&items, "slot");
        assert!(!slots.is_empty(), "should find at least one slot element");
    }

    #[test]
    fn named_slot_has_name_attr() {
        let source = include_str!("../../tests/fixtures/sample.html");
        let items = parse_and_extract(source);
        let slots = find_all_by_tag(&items, "slot");
        let named = slots.iter().find(|s| {
            s.metadata
                .html_attributes
                .iter()
                .any(|(n, v)| n == "name" && v.as_deref() == Some("sidebar-content"))
        });
        assert!(
            named.is_some(),
            "should find slot with name=sidebar-content"
        );
    }
}
