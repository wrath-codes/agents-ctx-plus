use ast_grep_core::Node;

pub(super) fn unquote_json_string(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        return serde_json::from_str::<String>(trimmed)
            .unwrap_or_else(|_| trimmed[1..trimmed.len() - 1].to_string());
    }

    trimmed.to_string()
}

pub(super) fn value_type_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    match node.kind().as_ref() {
        "string" => "string".to_string(),
        "number" => "number".to_string(),
        "true" | "false" => "boolean".to_string(),
        "null" => "null".to_string(),
        "object" => "object".to_string(),
        "array" => "array".to_string(),
        _ => node.kind().as_ref().to_string(),
    }
}

pub(super) fn path_join(prefix: &str, segment: &str) -> String {
    if is_simple_path_segment(segment) {
        if prefix.is_empty() {
            segment.to_string()
        } else {
            format!("{prefix}.{segment}")
        }
    } else {
        let escaped = escape_path_segment(segment);
        if prefix.is_empty() {
            format!("[\"{escaped}\"]")
        } else {
            format!("{prefix}[\"{escaped}\"]")
        }
    }
}

fn is_simple_path_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn escape_path_segment(segment: &str) -> String {
    segment.replace('\\', "\\\\").replace('"', "\\\"")
}
