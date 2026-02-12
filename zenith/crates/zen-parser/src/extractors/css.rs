//! `CSS` rich extractor.
//!
//! Extracts structurally significant elements from CSS stylesheets:
//! rule sets (class, id, element, attribute, pseudo, universal selectors),
//! `@media` queries, `@keyframes` animations, `@font-face` declarations,
//! `@import` statements, `@layer`, `@container`, `@supports`, `@scope`,
//! `@charset`, `@namespace`, `@page`, `@property`, `@counter-style`,
//! `@starting-style`, and CSS custom properties (`--vars`).
//! Also handles modern selectors: `:is()`, `:where()`, `:has()`, `:not()`,
//! native CSS nesting with `&`, and the universal `*` selector.

use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use crate::types::{CssMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

/// Extract all significant elements from a CSS stylesheet.
///
/// Walks the top-level stylesheet nodes collecting:
/// - Rule sets (class, id, element, attribute, pseudo, universal, nesting selectors)
/// - Modern pseudo-functions: `:is()`, `:where()`, `:has()`, `:not()`
/// - `@media` queries
/// - `@keyframes` animations
/// - `@font-face` declarations
/// - `@import` / `@charset` / `@namespace` statements
/// - `@layer`, `@container`, `@supports`, `@scope` at-rules
/// - Generic at-rules: `@page`, `@property`, `@counter-style`, `@starting-style`
/// - CSS custom properties (`--*`) inside `:root`
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = Vec::new();
    let root_node = root.root();
    collect_nodes(&root_node, &mut items, None);
    Ok(items)
}

fn collect_nodes<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    parent_context: Option<&str>,
) {
    let kind = node.kind();
    let recurse = match kind.as_ref() {
        "rule_set" => {
            process_rule_set(node, items, parent_context);
            false // leaf — no further recursion needed
        }
        "media_statement" => {
            process_media_statement(node, items);
            false // handles its own recursion into block
        }
        "keyframes_statement" => {
            process_keyframes(node, items);
            false
        }
        "import_statement" => {
            process_import(node, items);
            false
        }
        "charset_statement" => {
            process_charset(node, items);
            false
        }
        "namespace_statement" => {
            process_namespace(node, items);
            false
        }
        "supports_statement" => {
            process_supports(node, items);
            false // handles its own recursion
        }
        "at_rule" => {
            process_at_rule(node, items);
            false // handles its own recursion
        }
        "scope_statement" => {
            process_scope(node, items);
            false // handles its own recursion
        }
        _ => true, // recurse for stylesheet, etc.
    };

    if recurse {
        let children: Vec<_> = node.children().collect();
        for child in &children {
            collect_nodes(child, items, parent_context);
        }
    }
}

// ── Rule set processing ────────────────────────────────────────────

fn process_rule_set<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    parent_context: Option<&str>,
) {
    let children: Vec<_> = node.children().collect();

    let selector_text = children
        .iter()
        .find(|c| c.kind().as_ref() == "selectors")
        .map(|s| s.text().to_string())
        .unwrap_or_default();

    let properties = extract_properties(node);
    let custom_props = extract_custom_properties(node);

    // Emit custom properties as individual items
    for (prop_name, prop_value) in &custom_props {
        let mut metadata = SymbolMetadata::default();
        metadata.set_selector(selector_text.clone());
        metadata.mark_custom_property();

        items.push(ParsedItem {
            kind: SymbolKind::Const,
            name: prop_name.clone(),
            signature: format!("{prop_name}: {prop_value}"),
            source: None,
            doc_comment: String::new(),
            start_line: node.start_pos().line() as u32 + 1,
            end_line: node.end_pos().line() as u32 + 1,
            visibility: Visibility::Public,
            metadata,
        });
    }

    // Determine the kind and name for the rule set
    let symbol_kind = classify_selector(&selector_text);
    let name = build_rule_name(&selector_text, parent_context);

    let signature = build_rule_signature(&selector_text, &properties);

    let mut metadata = SymbolMetadata::default();
    metadata.set_selector(selector_text);
    metadata.set_css_properties(properties);

    items.push(ParsedItem {
        kind: symbol_kind,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── @media processing ──────────────────────────────────────────────

fn process_media_statement<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let query = extract_media_query_text(&children);
    let name = format!("@media {query}");
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("media");
    metadata.set_media_query(query.clone());

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into the block to collect nested rules
    for child in &children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, Some(&query));
            }
        }
    }
}

// ── @keyframes processing ──────────────────────────────────────────

fn process_keyframes<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let anim_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "keyframes_name")
        .map_or_else(|| "unnamed".to_string(), |n| n.text().to_string());

    let name = format!("@keyframes {anim_name}");
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("keyframes");

    items.push(ParsedItem {
        kind: SymbolKind::Function,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── @import processing ─────────────────────────────────────────────

fn process_import<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let url = extract_url_from_node(node).unwrap_or_else(|| "unknown".to_string());

    let name = url.clone();
    let signature = format!("@import url('{url}')");

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("import");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── @charset processing ────────────────────────────────────────────

fn process_charset<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();
    let charset = children
        .iter()
        .find(|c| c.kind().as_ref() == "string_value")
        .and_then(|sv| {
            sv.children()
                .find(|c| c.kind().as_ref() == "string_content")
                .map(|sc| sc.text().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let name = format!("@charset {charset}");
    let signature = format!("@charset \"{charset}\"");

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("charset");

    items.push(ParsedItem {
        kind: SymbolKind::Const,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── @namespace processing ──────────────────────────────────────────

fn process_namespace<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();
    let ns_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "namespace_name")
        .map_or_else(|| "default".to_string(), |n| n.text().to_string());

    let url = extract_url_from_node(node).unwrap_or_else(|| "unknown".to_string());

    let name = format!("@namespace {ns_name}");
    let signature = format!("@namespace {ns_name} url({url})");

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("namespace");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: Some(node.text().to_string()),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

// ── @supports processing ──────────────────────────────────────────

fn process_supports<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let query = children
        .iter()
        .find(|c| c.kind().as_ref() == "feature_query")
        .map(|fq| fq.text().to_string())
        .unwrap_or_default();

    let name = format!("@supports {query}");
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("supports");
    metadata.set_media_query(query.clone());

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules
    for child in &children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, Some(&query));
            }
        }
    }
}

// ── @scope processing ──────────────────────────────────────────────

fn process_scope<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    // Build scope description from the selectors between parentheses.
    // AST: @scope ( .from-selector ) to ( .to-selector ) block
    // We extract the class_selector children to build the name.
    let mut from_sel = None;
    let mut to_sel = None;
    let mut seen_to = false;
    let mut seen_first_close = false;

    for child in &children {
        let k = child.kind();
        match k.as_ref() {
            "to" => seen_to = true,
            ")" => seen_first_close = true,
            "class_selector"
            | "id_selector"
            | "tag_name"
            | "pseudo_class_selector"
            | "universal_selector" => {
                let text = child.text().to_string();
                if seen_to {
                    to_sel = Some(text);
                } else if seen_first_close {
                    // after first close paren = unexpected, ignore
                } else {
                    from_sel = Some(text);
                }
            }
            _ => {}
        }
    }

    let name = match (&from_sel, &to_sel) {
        (Some(f), Some(t)) => format!("@scope ({f}) to ({t})"),
        (Some(f), None) => format!("@scope ({f})"),
        _ => "@scope".to_string(),
    };
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("scope");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules
    for child in &children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, from_sel.as_deref());
            }
        }
    }
}

// ── Generic @rule processing (font-face, layer, container, etc.) ──

fn process_at_rule<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let children: Vec<_> = node.children().collect();

    let at_keyword = children
        .iter()
        .find(|c| c.kind().as_ref() == "at_keyword")
        .map(|k| k.text().to_string())
        .unwrap_or_default();

    // Strip the leading '@'
    let rule_name = at_keyword.trim_start_matches('@');

    match rule_name {
        "font-face" => process_font_face(node, items, &children),
        "layer" => process_layer(node, items, &children),
        "container" => process_container(node, items, &children),
        _ => process_generic_at_rule(node, items, rule_name, &children),
    }
}

fn process_font_face<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    children: &[Node<D>],
) {
    let properties = extract_properties_from_block(children);

    // Try to extract font-family name from declarations
    let font_family = properties
        .iter()
        .find(|p| p.starts_with("font-family:"))
        .map_or_else(
            || "unnamed".to_string(),
            |p| p.trim_start_matches("font-family:").trim().to_string(),
        );

    let name = format!("@font-face {font_family}");
    let signature = format!("@font-face {{ font-family: {font_family} }}");

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("font-face");
    metadata.set_css_properties(properties);

    items.push(ParsedItem {
        kind: SymbolKind::Struct,
        name,
        signature,
        source: extract_source_limited(node, 10),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });
}

fn process_layer<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    children: &[Node<D>],
) {
    let layer_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "keyword_query")
        .map_or_else(|| "anonymous".to_string(), |n| n.text().to_string());

    let name = format!("@layer {layer_name}");
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("layer");

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules
    for child in children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, Some(&layer_name));
            }
        }
    }
}

fn process_container<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    children: &[Node<D>],
) {
    let container_name = children
        .iter()
        .find(|c| c.kind().as_ref() == "keyword_query")
        .map(|n| n.text().to_string());

    let query = children
        .iter()
        .find(|c| c.kind().as_ref() == "feature_query")
        .map(|fq| fq.text().to_string())
        .unwrap_or_default();

    let name = container_name.as_ref().map_or_else(
        || format!("@container {query}"),
        |cn| format!("@container {cn} {query}"),
    );
    let signature = name.clone();

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name("container");
    metadata.set_media_query(query);

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 20),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules
    for child in children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                collect_nodes(bc, items, None);
            }
        }
    }
}

fn process_generic_at_rule<D: ast_grep_core::Doc>(
    node: &Node<D>,
    items: &mut Vec<ParsedItem>,
    rule_name: &str,
    children: &[Node<D>],
) {
    // Extract optional keyword_query for named at-rules
    // (e.g., `@property --my-color`, `@counter-style thumbs`)
    let keyword = children
        .iter()
        .find(|c| c.kind().as_ref() == "keyword_query")
        .map(|n| n.text().to_string());

    let name = keyword.as_ref().map_or_else(
        || format!("@{rule_name}"),
        |kw| format!("@{rule_name} {kw}"),
    );

    let text = node.text().to_string();
    // Take the first line as signature
    let signature = text.lines().next().unwrap_or(&text).to_string();

    let properties = extract_properties_from_block(children);

    let mut metadata = SymbolMetadata::default();
    metadata.set_at_rule_name(rule_name.to_string());
    metadata.set_css_properties(properties);

    items.push(ParsedItem {
        kind: SymbolKind::Module,
        name,
        signature,
        source: extract_source_limited(node, 10),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    });

    // Recurse into block for nested rules (e.g., `@starting-style`)
    for child in children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                if bc.kind().as_ref() == "rule_set" {
                    collect_nodes(bc, items, keyword.as_deref());
                }
            }
        }
    }
}

// ── Helper functions ───────────────────────────────────────────────

/// Classify a selector string into a `SymbolKind`.
fn classify_selector(selector: &str) -> SymbolKind {
    let trimmed = selector.trim();
    if trimmed.starts_with('#') {
        // ID selector → treated as a unique identifier
        SymbolKind::Static
    } else if trimmed.starts_with('.') {
        // Class selector
        SymbolKind::Class
    } else if trimmed.starts_with(':') {
        // Pseudo-class/element on root-level
        SymbolKind::Class
    } else if trimmed.starts_with('[') {
        // Attribute selector
        SymbolKind::Class
    } else if trimmed.contains(',') {
        // Grouped selectors
        SymbolKind::Class
    } else {
        // Element/type selector or complex selector
        SymbolKind::Class
    }
}

/// Build a human-readable rule name from a selector.
fn build_rule_name(selector: &str, parent_context: Option<&str>) -> String {
    let trimmed = selector.trim();
    parent_context.map_or_else(
        || trimmed.to_string(),
        |ctx| {
            let short_ctx: String = ctx.chars().take(30).collect();
            format!("{trimmed} @{short_ctx}")
        },
    )
}

/// Build a signature showing selector + property names.
fn build_rule_signature(selector: &str, properties: &[String]) -> String {
    use std::fmt::Write;
    let mut sig = format!("{selector} {{");
    for (i, prop) in properties.iter().enumerate() {
        if i >= 5 {
            let _ = write!(sig, " /* +{} more */", properties.len() - 5);
            break;
        }
        let _ = write!(sig, " {prop};");
    }
    sig.push_str(" }");
    sig
}

/// Extract property declarations from a rule set's block.
fn extract_properties<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let children: Vec<_> = node.children().collect();
    extract_properties_from_block_node(&children)
}

/// Extract properties from children that include a block.
fn extract_properties_from_block<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Vec<String> {
    extract_properties_from_block_node(children)
}

fn extract_properties_from_block_node<D: ast_grep_core::Doc>(children: &[Node<D>]) -> Vec<String> {
    let mut properties = Vec::new();
    for child in children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                if bc.kind().as_ref() == "declaration" {
                    let prop_text = bc.text().to_string();
                    // Remove trailing semicolon and trim
                    let clean = prop_text.trim().trim_end_matches(';').trim().to_string();
                    properties.push(clean);
                }
            }
        }
    }
    properties
}

/// Extract CSS custom property declarations (`--*`) from a rule set.
///
/// Returns `(property_name, value)` pairs.
fn extract_custom_properties<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<(String, String)> {
    let mut custom_props = Vec::new();
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "block" {
            let block_children: Vec<_> = child.children().collect();
            for bc in &block_children {
                if bc.kind().as_ref() == "declaration" {
                    let decl_children: Vec<_> = bc.children().collect();
                    let prop_name = decl_children
                        .iter()
                        .find(|c| c.kind().as_ref() == "property_name")
                        .map(|n| n.text().to_string());
                    if let Some(ref pn) = prop_name
                        && pn.starts_with("--")
                    {
                        // Extract value (everything after the colon)
                        let full_text = bc.text().to_string();
                        let value = full_text
                            .find(':')
                            .map(|pos| {
                                full_text[pos + 1..]
                                    .trim()
                                    .trim_end_matches(';')
                                    .trim()
                                    .to_string()
                            })
                            .unwrap_or_default();
                        custom_props.push((pn.clone(), value));
                    }
                }
            }
        }
    }
    custom_props
}

/// Extract a URL from a node containing a `call_expression` with `url(...)`.
fn extract_url_from_node<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "call_expression" {
            let call_children: Vec<_> = child.children().collect();
            let args = call_children
                .iter()
                .find(|c| c.kind().as_ref() == "arguments")?;
            let args_children: Vec<_> = args.children().collect();
            // Try string_value first, then plain_value
            let url = args_children
                .iter()
                .find(|c| c.kind().as_ref() == "string_value")
                .and_then(|sv| {
                    sv.children()
                        .find(|c| c.kind().as_ref() == "string_content")
                        .map(|sc| sc.text().to_string())
                })
                .or_else(|| {
                    args_children
                        .iter()
                        .find(|c| c.kind().as_ref() == "plain_value")
                        .map(|pv| pv.text().to_string())
                });
            return url;
        }
    }
    None
}

/// Extract the media query text from a `media_statement` node's children.
fn extract_media_query_text<D: ast_grep_core::Doc>(children: &[Node<D>]) -> String {
    // Collect all feature_query and keyword_query children
    let mut parts = Vec::new();
    for child in children {
        let k = child.kind();
        match k.as_ref() {
            "feature_query" | "keyword_query" => {
                parts.push(child.text().to_string());
            }
            _ => {}
        }
    }
    parts.join(" ")
}

/// Limit source extraction to `max_lines` lines.
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
            "{truncated}\n    /* ... ({} more lines) */",
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
        let root = SupportLang::Css.ast_grep(source);
        extract(&root).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items.iter().find(|i| i.name == name).unwrap_or_else(|| {
            let names: Vec<_> = items.iter().map(|i| &i.name).collect();
            panic!("should find item named '{name}', available: {names:?}")
        })
    }

    fn find_all_by_at_rule<'a>(items: &'a [ParsedItem], rule: &str) -> Vec<&'a ParsedItem> {
        items
            .iter()
            .filter(|i| i.metadata.at_rule_name.as_deref() == Some(rule))
            .collect()
    }

    // ── Import tests ───────────────────────────────────────────────

    #[test]
    fn import_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let imports = find_all_by_at_rule(&items, "import");
        assert_eq!(imports.len(), 2, "should find 2 @import statements");
    }

    #[test]
    fn import_url_as_name() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "reset.css");
        assert_eq!(i.kind, SymbolKind::Module);
        assert_eq!(i.metadata.at_rule_name.as_deref(), Some("import"));
    }

    #[test]
    fn import_nested_url() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "theme/dark.css");
        assert_eq!(i.kind, SymbolKind::Module);
    }

    // ── Charset tests ──────────────────────────────────────────────

    #[test]
    fn charset_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "@charset UTF-8");
        assert_eq!(c.kind, SymbolKind::Const);
        assert_eq!(c.metadata.at_rule_name.as_deref(), Some("charset"));
    }

    // ── Namespace tests ────────────────────────────────────────────

    #[test]
    fn namespace_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let n = find_by_name(&items, "@namespace svg");
        assert_eq!(n.kind, SymbolKind::Module);
        assert_eq!(n.metadata.at_rule_name.as_deref(), Some("namespace"));
    }

    // ── Custom property tests ──────────────────────────────────────

    #[test]
    fn custom_properties_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let cp = find_by_name(&items, "--primary-color");
        assert_eq!(cp.kind, SymbolKind::Const);
        assert!(cp.metadata.is_custom_property);
        assert!(cp.signature.contains("#3498db"));
    }

    #[test]
    fn custom_property_spacing() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let cp = find_by_name(&items, "--spacing-md");
        assert_eq!(cp.kind, SymbolKind::Const);
        assert!(cp.signature.contains("1rem"));
    }

    #[test]
    fn custom_property_count() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let vars: Vec<_> = items
            .iter()
            .filter(|i| i.metadata.is_custom_property)
            .collect();
        assert_eq!(vars.len(), 7, "should find 7 CSS custom properties");
    }

    // ── Element selector tests ─────────────────────────────────────

    #[test]
    fn element_selector_body() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let b = find_by_name(&items, "body");
        assert_eq!(b.kind, SymbolKind::Class);
        assert_eq!(b.metadata.selector.as_deref(), Some("body"));
    }

    #[test]
    fn element_selector_properties() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let b = find_by_name(&items, "body");
        assert!(
            b.metadata
                .css_properties
                .iter()
                .any(|p| p.starts_with("margin")),
            "body should have margin property: {:?}",
            b.metadata.css_properties
        );
    }

    // ── Class selector tests ───────────────────────────────────────

    #[test]
    fn class_selector_card() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, ".card");
        assert_eq!(c.kind, SymbolKind::Class);
        assert_eq!(c.metadata.selector.as_deref(), Some(".card"));
    }

    #[test]
    fn class_selector_bem_pattern() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let ct = find_by_name(&items, ".card__title");
        assert_eq!(ct.kind, SymbolKind::Class);
    }

    #[test]
    fn class_selector_modifier() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let bp = find_by_name(&items, ".btn--primary");
        assert_eq!(bp.kind, SymbolKind::Class);
    }

    // ── ID selector tests ──────────────────────────────────────────

    #[test]
    fn id_selector_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let h = find_by_name(&items, "#main-header");
        assert_eq!(h.kind, SymbolKind::Static);
        assert_eq!(h.metadata.selector.as_deref(), Some("#main-header"));
    }

    #[test]
    fn id_selector_sidebar() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "#sidebar");
        assert_eq!(s.kind, SymbolKind::Static);
    }

    // ── Pseudo-class tests ─────────────────────────────────────────

    #[test]
    fn pseudo_class_hover() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let h = find_by_name(&items, "a:hover");
        assert_eq!(h.kind, SymbolKind::Class);
    }

    #[test]
    fn pseudo_class_focus() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, ".btn:focus");
        assert_eq!(f.kind, SymbolKind::Class);
    }

    // ── Pseudo-element tests ───────────────────────────────────────

    #[test]
    fn pseudo_element_first_line() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "p::first-line");
        assert_eq!(p.kind, SymbolKind::Class);
    }

    #[test]
    fn pseudo_element_after() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, ".tooltip::after");
        assert_eq!(t.kind, SymbolKind::Class);
    }

    // ── Combinator tests ───────────────────────────────────────────

    #[test]
    fn descendant_selector() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let d = find_by_name(&items, ".parent .child");
        assert_eq!(d.kind, SymbolKind::Class);
    }

    #[test]
    fn child_selector() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, ".parent > .direct-child");
        assert_eq!(c.kind, SymbolKind::Class);
    }

    #[test]
    fn adjacent_sibling_selector() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let a = find_by_name(&items, ".sibling + .adjacent");
        assert_eq!(a.kind, SymbolKind::Class);
    }

    #[test]
    fn general_sibling_selector() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let g = find_by_name(&items, ".sibling ~ .general");
        assert_eq!(g.kind, SymbolKind::Class);
    }

    // ── Multiple selectors test ────────────────────────────────────

    #[test]
    fn multiple_selectors() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "h1, h2, h3, h4");
        assert_eq!(m.kind, SymbolKind::Class);
    }

    // ── @media tests ───────────────────────────────────────────────

    #[test]
    fn media_query_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let medias = find_all_by_at_rule(&items, "media");
        assert_eq!(medias.len(), 3, "should find 3 @media statements");
    }

    #[test]
    fn media_query_name() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "@media (max-width: 768px)");
        assert_eq!(m.kind, SymbolKind::Module);
        assert_eq!(m.metadata.at_rule_name.as_deref(), Some("media"));
    }

    #[test]
    fn media_query_nested_rules() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        // Nested rules inside @media should have parent context in name
        let nested: Vec<_> = items
            .iter()
            .filter(|i| i.name.contains("@(max-width:"))
            .collect();
        assert!(!nested.is_empty(), "should find nested rules inside @media");
    }

    #[test]
    fn media_print_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let m = find_by_name(&items, "@media print");
        assert_eq!(m.kind, SymbolKind::Module);
    }

    // ── @keyframes tests ───────────────────────────────────────────

    #[test]
    fn keyframes_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let kf = find_all_by_at_rule(&items, "keyframes");
        assert_eq!(kf.len(), 3, "should find 3 @keyframes");
    }

    #[test]
    fn keyframes_fade_in() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let k = find_by_name(&items, "@keyframes fadeIn");
        assert_eq!(k.kind, SymbolKind::Function);
        assert_eq!(k.metadata.at_rule_name.as_deref(), Some("keyframes"));
    }

    #[test]
    fn keyframes_slide_in() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let k = find_by_name(&items, "@keyframes slideIn");
        assert_eq!(k.kind, SymbolKind::Function);
    }

    #[test]
    fn keyframes_pulse() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let k = find_by_name(&items, "@keyframes pulse");
        assert_eq!(k.kind, SymbolKind::Function);
    }

    // ── @font-face tests ───────────────────────────────────────────

    #[test]
    fn font_face_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let ff = find_all_by_at_rule(&items, "font-face");
        assert_eq!(ff.len(), 2, "should find 2 @font-face declarations");
    }

    #[test]
    fn font_face_kind() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let ff = find_all_by_at_rule(&items, "font-face");
        for f in &ff {
            assert_eq!(f.kind, SymbolKind::Struct);
        }
    }

    #[test]
    fn font_face_has_properties() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let ff = find_all_by_at_rule(&items, "font-face");
        let first = ff.first().expect("should have at least one @font-face");
        assert!(
            first
                .metadata
                .css_properties
                .iter()
                .any(|p| p.contains("font-family")),
            "font-face should have font-family property: {:?}",
            first.metadata.css_properties
        );
    }

    // ── @layer tests ───────────────────────────────────────────────

    #[test]
    fn layer_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let layers = find_all_by_at_rule(&items, "layer");
        assert_eq!(layers.len(), 2, "should find 2 @layer declarations");
    }

    #[test]
    fn layer_base() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let l = find_by_name(&items, "@layer base");
        assert_eq!(l.kind, SymbolKind::Module);
        assert_eq!(l.metadata.at_rule_name.as_deref(), Some("layer"));
    }

    #[test]
    fn layer_utilities() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let l = find_by_name(&items, "@layer utilities");
        assert_eq!(l.kind, SymbolKind::Module);
    }

    #[test]
    fn layer_nested_rules() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        // Rules inside @layer should have parent context
        let nested: Vec<_> = items
            .iter()
            .filter(|i| i.name.contains("@base") || i.name.contains("@utilities"))
            .collect();
        assert!(!nested.is_empty(), "should find nested rules inside @layer");
    }

    // ── @container tests ───────────────────────────────────────────

    #[test]
    fn container_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let containers = find_all_by_at_rule(&items, "container");
        assert_eq!(containers.len(), 1, "should find 1 @container query");
    }

    #[test]
    fn container_kind() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let containers = find_all_by_at_rule(&items, "container");
        let c = containers.first().expect("should have @container");
        assert_eq!(c.kind, SymbolKind::Module);
    }

    // ── @supports tests ────────────────────────────────────────────

    #[test]
    fn supports_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let supports = find_all_by_at_rule(&items, "supports");
        assert_eq!(supports.len(), 2, "should find 2 @supports statements");
    }

    #[test]
    fn supports_grid() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "@supports (display: grid)");
        assert_eq!(s.kind, SymbolKind::Module);
        assert_eq!(s.metadata.at_rule_name.as_deref(), Some("supports"));
    }

    // ── Complex selector tests ─────────────────────────────────────

    #[test]
    fn complex_hover_selector() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, ".card:hover .card__title");
        assert_eq!(c.kind, SymbolKind::Class);
    }

    #[test]
    fn attribute_selector() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let a = find_by_name(&items, "[data-tooltip]");
        assert_eq!(a.kind, SymbolKind::Class);
    }

    // ── Line number tests ──────────────────────────────────────────

    #[test]
    fn line_numbers_monotonic() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        for item in &items {
            assert!(
                item.end_line >= item.start_line,
                "end_line should be >= start_line for '{}': {} < {}",
                item.name,
                item.end_line,
                item.start_line
            );
        }
    }

    #[test]
    fn line_numbers_positive() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        for item in &items {
            assert!(
                item.start_line > 0,
                "start_line should be > 0 for '{}'",
                item.name
            );
        }
    }

    // ── Signature tests ────────────────────────────────────────────

    #[test]
    fn rule_signature_includes_selector() {
        let source = ".card { display: flex; }";
        let items = parse_and_extract(source);
        let c = find_by_name(&items, ".card");
        assert!(
            c.signature.starts_with(".card"),
            "signature should start with selector: {}",
            c.signature
        );
    }

    #[test]
    fn rule_signature_includes_properties() {
        let source = ".card { display: flex; color: red; }";
        let items = parse_and_extract(source);
        let c = find_by_name(&items, ".card");
        assert!(
            c.signature.contains("display: flex"),
            "signature should include properties: {}",
            c.signature
        );
    }

    // ── Source extraction tests ─────────────────────────────────────

    #[test]
    fn source_present_for_rule() {
        let source = ".test { color: red; }";
        let items = parse_and_extract(source);
        let t = find_by_name(&items, ".test");
        assert!(t.source.is_some(), "source should be present");
    }

    // ── Inline tests (no fixture) ──────────────────────────────────

    #[test]
    fn simple_class_rule() {
        let items = parse_and_extract(".btn { padding: 8px; }");
        let b = find_by_name(&items, ".btn");
        assert_eq!(b.kind, SymbolKind::Class);
        assert!(
            b.metadata
                .css_properties
                .iter()
                .any(|p| p.contains("padding"))
        );
    }

    #[test]
    fn simple_id_rule() {
        let items = parse_and_extract("#app { width: 100%; }");
        let a = find_by_name(&items, "#app");
        assert_eq!(a.kind, SymbolKind::Static);
    }

    #[test]
    fn simple_media() {
        let items = parse_and_extract("@media (max-width: 600px) { .box { display: block; } }");
        let m = find_by_name(&items, "@media (max-width: 600px)");
        assert_eq!(m.kind, SymbolKind::Module);
    }

    #[test]
    fn simple_keyframes() {
        let items = parse_and_extract("@keyframes spin { to { transform: rotate(360deg); } }");
        let k = find_by_name(&items, "@keyframes spin");
        assert_eq!(k.kind, SymbolKind::Function);
    }

    #[test]
    fn simple_import() {
        let items = parse_and_extract("@import url('base.css');");
        let i = find_by_name(&items, "base.css");
        assert_eq!(i.kind, SymbolKind::Module);
    }

    #[test]
    fn simple_charset() {
        let items = parse_and_extract("@charset \"UTF-8\";");
        let c = find_by_name(&items, "@charset UTF-8");
        assert_eq!(c.kind, SymbolKind::Const);
    }

    #[test]
    fn simple_custom_property() {
        let items = parse_and_extract(":root { --gap: 10px; }");
        let cp = find_by_name(&items, "--gap");
        assert_eq!(cp.kind, SymbolKind::Const);
        assert!(cp.metadata.is_custom_property);
    }

    // ── Universal selector tests ──────────────────────────────────

    #[test]
    fn universal_selector_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let u = find_by_name(&items, "*");
        assert_eq!(u.kind, SymbolKind::Class);
        assert_eq!(u.metadata.selector.as_deref(), Some("*"));
    }

    #[test]
    fn universal_selector_inline() {
        let items = parse_and_extract("* { margin: 0; }");
        let u = find_by_name(&items, "*");
        assert_eq!(u.kind, SymbolKind::Class);
    }

    // ── Modern pseudo-function tests ───────────────────────────────

    #[test]
    fn is_pseudo_function() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, ":is(h1, h2, h3)");
        assert_eq!(i.kind, SymbolKind::Class);
    }

    #[test]
    fn where_pseudo_function() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let w = find_by_name(&items, ":where(.card, .panel)");
        assert_eq!(w.kind, SymbolKind::Class);
    }

    #[test]
    fn has_pseudo_function() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let h = find_by_name(&items, "article:has(> img)");
        assert_eq!(h.kind, SymbolKind::Class);
    }

    #[test]
    fn not_pseudo_function() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let n = find_by_name(&items, ":not(.active)");
        assert_eq!(n.kind, SymbolKind::Class);
    }

    // ── Native CSS nesting tests ───────────────────────────────────

    #[test]
    fn nesting_parent_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let n = find_by_name(&items, ".nav");
        assert_eq!(n.kind, SymbolKind::Class);
    }

    #[test]
    fn nesting_inline() {
        let items = parse_and_extract(".card { color: black; & .title { font-size: 2rem; } }");
        let c = find_by_name(&items, ".card");
        assert_eq!(c.kind, SymbolKind::Class);
        // Nested rule should also be extracted (it's a rule_set inside block)
    }

    // ── @page tests ────────────────────────────────────────────────

    #[test]
    fn page_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "@page");
        assert_eq!(p.kind, SymbolKind::Module);
        assert_eq!(p.metadata.at_rule_name.as_deref(), Some("page"));
    }

    #[test]
    fn page_has_properties() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "@page");
        assert!(
            p.metadata
                .css_properties
                .iter()
                .any(|prop| prop.contains("margin")),
            "page should have margin property: {:?}",
            p.metadata.css_properties
        );
    }

    // ── @property tests ────────────────────────────────────────────

    #[test]
    fn property_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "@property --gradient-angle");
        assert_eq!(p.kind, SymbolKind::Module);
        assert_eq!(p.metadata.at_rule_name.as_deref(), Some("property"));
    }

    #[test]
    fn property_has_declarations() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "@property --gradient-angle");
        assert!(
            p.metadata
                .css_properties
                .iter()
                .any(|prop| prop.contains("syntax")),
            "property should have syntax declaration: {:?}",
            p.metadata.css_properties
        );
    }

    // ── @counter-style tests ───────────────────────────────────────

    #[test]
    fn counter_style_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let cs = find_by_name(&items, "@counter-style thumbs");
        assert_eq!(cs.kind, SymbolKind::Module);
        assert_eq!(cs.metadata.at_rule_name.as_deref(), Some("counter-style"));
    }

    #[test]
    fn counter_style_has_properties() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let cs = find_by_name(&items, "@counter-style thumbs");
        assert!(
            cs.metadata
                .css_properties
                .iter()
                .any(|prop| prop.contains("system")),
            "counter-style should have system property: {:?}",
            cs.metadata.css_properties
        );
    }

    // ── @scope tests ───────────────────────────────────────────────

    #[test]
    fn scope_with_to_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "@scope (.card) to (.card__body)");
        assert_eq!(s.kind, SymbolKind::Module);
        assert_eq!(s.metadata.at_rule_name.as_deref(), Some("scope"));
    }

    #[test]
    fn scope_without_to_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let s = find_by_name(&items, "@scope (.hero)");
        assert_eq!(s.kind, SymbolKind::Module);
        assert_eq!(s.metadata.at_rule_name.as_deref(), Some("scope"));
    }

    #[test]
    fn scope_count() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let scopes = find_all_by_at_rule(&items, "scope");
        assert_eq!(scopes.len(), 2, "should find 2 @scope statements");
    }

    #[test]
    fn scope_nested_rules() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        // Rules inside @scope should have parent context
        let nested: Vec<_> = items
            .iter()
            .filter(|i| i.name.contains("@.card") || i.name.contains("@.hero"))
            .collect();
        assert!(!nested.is_empty(), "should find nested rules inside @scope");
    }

    #[test]
    fn scope_inline() {
        let items = parse_and_extract("@scope (.panel) { h2 { font-size: 1.5rem; } }");
        let s = find_by_name(&items, "@scope (.panel)");
        assert_eq!(s.kind, SymbolKind::Module);
    }

    // ── @starting-style tests ──────────────────────────────────────

    #[test]
    fn starting_style_extracted() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        let ss = find_by_name(&items, "@starting-style");
        assert_eq!(ss.kind, SymbolKind::Module);
        assert_eq!(ss.metadata.at_rule_name.as_deref(), Some("starting-style"));
    }

    #[test]
    fn starting_style_nested_rules() {
        let source = include_str!("../../tests/fixtures/sample.css");
        let items = parse_and_extract(source);
        // The .fade-in rule inside @starting-style should be extracted
        let fade_in: Vec<_> = items
            .iter()
            .filter(|i| i.name.contains("fade-in"))
            .collect();
        assert!(
            !fade_in.is_empty(),
            "should find .fade-in nested inside @starting-style"
        );
    }

    // ── Inline tests for new constructs ────────────────────────────

    #[test]
    fn inline_page() {
        let items = parse_and_extract("@page { margin: 1in; }");
        let p = find_by_name(&items, "@page");
        assert_eq!(p.kind, SymbolKind::Module);
    }

    #[test]
    fn inline_property() {
        let items = parse_and_extract(
            "@property --my-bg { syntax: '<color>'; inherits: false; initial-value: white; }",
        );
        let p = find_by_name(&items, "@property --my-bg");
        assert_eq!(p.kind, SymbolKind::Module);
    }

    #[test]
    fn inline_counter_style() {
        let items = parse_and_extract("@counter-style stars { system: cyclic; symbols: \"★\"; }");
        let cs = find_by_name(&items, "@counter-style stars");
        assert_eq!(cs.kind, SymbolKind::Module);
    }

    #[test]
    fn inline_scope() {
        let items = parse_and_extract("@scope (.wrapper) to (.inner) { div { padding: 1rem; } }");
        let s = find_by_name(&items, "@scope (.wrapper) to (.inner)");
        assert_eq!(s.kind, SymbolKind::Module);
    }

    #[test]
    fn inline_starting_style() {
        let items = parse_and_extract("@starting-style { .box { scale: 0; } }");
        let ss = find_by_name(&items, "@starting-style");
        assert_eq!(ss.kind, SymbolKind::Module);
    }

    #[test]
    fn empty_stylesheet() {
        let items = parse_and_extract("");
        assert!(items.is_empty());
    }

    #[test]
    fn comment_only_stylesheet() {
        let items = parse_and_extract("/* just a comment */");
        assert!(items.is_empty());
    }
}
