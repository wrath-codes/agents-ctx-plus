use ast_grep_core::Node;

pub fn normalize_type_node<D: ast_grep_core::Doc>(node: Option<Node<D>>) -> Option<String> {
    node.map(|type_node| normalize_type_text(type_node.text().as_ref()))
        .filter(|text| !text.is_empty())
}

pub fn normalize_type_text(text: &str) -> String {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    normalize_expr(&compact)
}

fn normalize_expr(expr: &str) -> String {
    let trimmed = strip_outer_parens(expr);

    if let Some(inner) = trimmed.strip_prefix('?') {
        return format!("?{}", normalize_expr(inner));
    }

    let union_parts = split_top_level(trimmed, '|');
    if union_parts.len() > 1 {
        let mut normalized = union_parts
            .iter()
            .map(|p| normalize_expr(p))
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();
        return normalized.join("|");
    }

    let intersection_parts = split_top_level(trimmed, '&');
    if intersection_parts.len() > 1 {
        let mut normalized = intersection_parts
            .iter()
            .map(|p| normalize_expr(p))
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();
        return normalized.join("&");
    }

    trimmed.to_string()
}

fn split_top_level(input: &str, delim: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0_usize;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if ch == delim && depth == 0 => {
                let part = input[start..idx].trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        parts.push(tail.to_string());
    }

    if parts.is_empty() {
        vec![input.to_string()]
    } else {
        parts
    }
}

fn strip_outer_parens(input: &str) -> &str {
    if !input.starts_with('(') || !input.ends_with(')') {
        return input;
    }

    let mut depth = 0_i32;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && idx < input.len() - 1 {
                    return input;
                }
            }
            _ => {}
        }
    }

    &input[1..input.len() - 1]
}
