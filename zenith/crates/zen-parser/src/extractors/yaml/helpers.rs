use ast_grep_core::Node;

pub(super) fn scalar_type_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let kind = node.kind();
    let kr = kind.as_ref();
    if kr == "block_node" || kr == "flow_node" {
        for child in node.children() {
            let child_kind = child.kind();
            let ckr = child_kind.as_ref();
            if ckr != "anchor" && ckr != "tag" {
                return scalar_type_name(&child);
            }
        }
    }

    if kr == "plain_scalar" && let Some(child) = node.children().next() {
        return scalar_type_name(&child);
    }

    match node.kind().as_ref() {
        "integer_scalar" | "float_scalar" => "number".to_string(),
        "boolean_scalar" => "boolean".to_string(),
        "null_scalar" => "null".to_string(),
        "timestamp_scalar" => "timestamp".to_string(),
        "string_scalar" | "single_quote_scalar" | "double_quote_scalar" | "block_scalar" => {
            "string".to_string()
        }
        "block_mapping" | "flow_mapping" => "object".to_string(),
        "block_sequence" | "flow_sequence" => "array".to_string(),
        "alias" => "alias".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn key_text<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let kind = node.kind();
    let kr = kind.as_ref();
    if kr == "block_node" || kr == "flow_node" {
        for child in node.children() {
            let child_kind = child.kind();
            let ckr = child_kind.as_ref();
            if ckr != "anchor" && ckr != "tag" {
                return key_text(&child);
            }
        }
    }

    normalize_scalar_text(&node.text())
}

pub(super) fn anchor_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    if node.kind().as_ref() != "anchor" {
        return None;
    }
    node.children()
        .find(|child| child.kind().as_ref() == "anchor_name")
        .map(|child| child.text().trim().to_string())
}

pub(super) fn alias_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    if node.kind().as_ref() != "alias" {
        return None;
    }
    node.children()
        .find(|child| child.kind().as_ref() == "alias_name")
        .map(|child| child.text().trim().to_string())
}

pub(super) fn normalize_tag(raw: &str) -> String {
    raw.trim().trim_start_matches('!').to_string()
}

pub(super) fn normalize_scalar_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

pub(super) fn path_join(prefix: &str, segment: &str) -> String {
    if is_simple_segment(segment) {
        if prefix.is_empty() {
            segment.to_string()
        } else {
            format!("{prefix}.{segment}")
        }
    } else {
        let escaped = segment.replace('\\', "\\\\").replace('"', "\\\"");
        if prefix.is_empty() {
            format!("[\"{escaped}\"]")
        } else {
            format!("{prefix}[\"{escaped}\"]")
        }
    }
}

fn is_simple_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
