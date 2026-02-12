use ast_grep_core::Node;

use crate::types::SymbolKind;

/// Classify a selector string into a `SymbolKind`.
pub(super) fn classify_selector(selector: &str) -> SymbolKind {
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
pub(super) fn build_rule_name(selector: &str, parent_context: Option<&str>) -> String {
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
pub(super) fn build_rule_signature(selector: &str, properties: &[String]) -> String {
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
pub(super) fn extract_properties<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let children: Vec<_> = node.children().collect();
    extract_properties_from_block_node(&children)
}

/// Extract properties from children that include a block.
pub(super) fn extract_properties_from_block<D: ast_grep_core::Doc>(
    children: &[Node<D>],
) -> Vec<String> {
    extract_properties_from_block_node(children)
}

pub(super) fn extract_properties_from_block_node<D: ast_grep_core::Doc>(
    children: &[Node<D>],
) -> Vec<String> {
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
pub(super) fn extract_custom_properties<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<(String, String)> {
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
pub(super) fn extract_url_from_node<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
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
pub(super) fn extract_media_query_text<D: ast_grep_core::Doc>(children: &[Node<D>]) -> String {
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
            "{truncated}\n    /* ... ({} more lines) */",
            lines.len() - max_lines
        ))
    }
}
