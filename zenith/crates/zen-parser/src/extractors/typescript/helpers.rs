use ast_grep_core::Node;

use crate::types::DocSections;

// ── JSDoc extraction ───────────────────────────────────────────────

pub(super) fn extract_jsdoc_before<D: ast_grep_core::Doc>(anchor: &Node<D>) -> String {
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

pub(super) fn parse_jsdoc_sections(doc: &str) -> DocSections {
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

pub(super) fn extract_ts_return_type<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
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

pub(super) fn extract_ts_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
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
