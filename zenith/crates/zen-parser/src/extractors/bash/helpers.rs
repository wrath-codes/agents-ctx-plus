use ast_grep_core::Node;

/// Extract source with line limit.
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
            "{truncated}\n    # ... ({} more lines)",
            lines.len() - max_lines
        ))
    }
}

/// Truncate text to a maximum length, appending `...` if needed.
pub(super) fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}
