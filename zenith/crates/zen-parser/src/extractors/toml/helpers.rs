use ast_grep_core::Node;

pub(super) fn is_key_kind(kind: &str) -> bool {
    matches!(kind, "bare_key" | "quoted_key" | "dotted_key")
}

pub(super) fn normalize_key(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return unescape_basic_string(inner);
    }
    if let Some(inner) = trimmed
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
    {
        return inner.to_string();
    }
    trimmed.to_string()
}

pub(super) fn key_parts<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    match node.kind().as_ref() {
        "dotted_key" => {
            let mut parts = Vec::new();
            for child in node.children() {
                let ck = child.kind();
                let cks = ck.as_ref();
                if matches!(cks, "bare_key" | "quoted_key") {
                    parts.push(normalize_key(&child.text()));
                }
            }
            if parts.is_empty() {
                split_dotted_key_preserving_quotes(&node.text())
            } else {
                parts
            }
        }
        _ => vec![normalize_key(&node.text())],
    }
}

pub(super) fn join_path(parent: &str, key_parts: &[String]) -> String {
    let key = key_parts.join(".");
    if parent.is_empty() {
        key
    } else {
        format!("{parent}.{key}")
    }
}

pub(super) fn toml_value_type<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    match node.kind().as_ref() {
        "string" => "string".to_string(),
        "integer" => "integer".to_string(),
        "float" => "float".to_string(),
        "boolean" => "boolean".to_string(),
        "local_date" => "local_date".to_string(),
        "local_time" => "local_time".to_string(),
        "local_date_time" => "local_date_time".to_string(),
        "offset_date_time" => "offset_date_time".to_string(),
        "array" => "array".to_string(),
        "inline_table" => "object".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn is_value_kind(kind: &str) -> bool {
    matches!(
        kind,
        "string"
            | "integer"
            | "float"
            | "boolean"
            | "local_date"
            | "local_time"
            | "local_date_time"
            | "offset_date_time"
            | "array"
            | "inline_table"
    )
}

pub(super) fn key_parts_from_pair_text(pair_text: &str) -> Option<Vec<String>> {
    let mut in_double = false;
    let mut in_single = false;
    let mut escaped = false;

    for (idx, ch) in pair_text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_double {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        match ch {
            '"' => in_double = true,
            '\'' => in_single = true,
            '=' => {
                let left = pair_text[..idx].trim();
                if left.is_empty() {
                    return None;
                }
                let parts = split_dotted_key_preserving_quotes(left);
                return if parts.is_empty() { None } else { Some(parts) };
            }
            _ => {}
        }
    }

    None
}

pub(super) fn key_parts_from_table_text(
    table_text: &str,
    is_array_table: bool,
) -> Option<Vec<String>> {
    let line = table_text.lines().next()?.trim();
    let inner = if is_array_table {
        line.strip_prefix("[[")?.strip_suffix("]]")?
    } else {
        line.strip_prefix('[')?.strip_suffix(']')?
    };
    let parts = split_dotted_key_preserving_quotes(inner.trim());
    if parts.is_empty() { None } else { Some(parts) }
}

pub(super) fn path_prefixes(path: &str) -> Vec<String> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() <= 1 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 1..parts.len() {
        out.push(parts[..i].join("."));
    }
    out
}

pub(super) fn normalized_scalar(kind: &str, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    match kind {
        "string" => normalize_toml_string(trimmed),
        "integer" => Some(normalize_integer(trimmed)),
        "float" => Some(normalize_float(trimmed)),
        "boolean" => Some(trimmed.to_ascii_lowercase()),
        "local_date" | "local_time" | "local_date_time" | "offset_date_time" => {
            Some(trimmed.to_string())
        }
        _ => None,
    }
}

pub(super) fn dependency_from_path(full_path: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = full_path.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    if parts[0] == "dependencies" {
        return Some(("cargo:dependencies".to_string(), parts[1].to_string()));
    }
    if parts[0] == "dev-dependencies" {
        return Some(("cargo:dev-dependencies".to_string(), parts[1].to_string()));
    }
    if parts[0] == "build-dependencies" {
        return Some(("cargo:build-dependencies".to_string(), parts[1].to_string()));
    }
    if parts.len() >= 3 && parts[0] == "workspace" && parts[1] == "dependencies" {
        return Some((
            "cargo:workspace-dependencies".to_string(),
            parts[2].to_string(),
        ));
    }

    if let Some((idx, _)) = parts
        .iter()
        .enumerate()
        .find(|(_, p)| **p == "dependencies")
        && idx > 0
        && parts[0] == "target"
        && parts.len() > idx + 1
    {
        return Some((
            "cargo:target-dependencies".to_string(),
            parts[idx + 1].to_string(),
        ));
    }

    if parts.len() >= 4 && parts[0] == "tool" && parts[1] == "poetry" && parts[2] == "dependencies"
    {
        return Some(("poetry:dependencies".to_string(), parts[3].to_string()));
    }
    if parts.len() >= 4
        && parts[0] == "tool"
        && parts[1] == "poetry"
        && parts[2] == "dev-dependencies"
    {
        return Some(("poetry:dev-dependencies".to_string(), parts[3].to_string()));
    }
    if parts.len() >= 6
        && parts[0] == "tool"
        && parts[1] == "poetry"
        && parts[2] == "group"
        && parts[4] == "dependencies"
    {
        return Some((format!("poetry:group:{}", parts[3]), parts[5].to_string()));
    }

    None
}

pub(super) fn pep508_req_from_string(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }

    let mut split = trimmed.len();
    for (i, ch) in trimmed.char_indices() {
        if ch.is_whitespace() || matches!(ch, '<' | '>' | '=' | '!' | '~' | '^' | '@') {
            split = i;
            break;
        }
    }
    let name = trimmed[..split].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let req = trimmed[split..].trim().to_string();
    Some((name, req))
}

fn normalize_toml_string(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed
        .strip_prefix("\"\"\"")
        .and_then(|s| s.strip_suffix("\"\"\""))
    {
        return Some(unescape_basic_string(inner));
    }
    if let Some(inner) = trimmed
        .strip_prefix("'''")
        .and_then(|s| s.strip_suffix("'''"))
    {
        return Some(inner.to_string());
    }
    if let Some(inner) = trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Some(unescape_basic_string(inner));
    }
    if let Some(inner) = trimmed
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
    {
        return Some(inner.to_string());
    }
    None
}

fn normalize_integer(raw: &str) -> String {
    let no_underscores = raw.replace('_', "");
    no_underscores
        .strip_prefix('+')
        .map_or_else(|| no_underscores.clone(), std::string::ToString::to_string)
}

fn normalize_float(raw: &str) -> String {
    let mut out = raw.replace('_', "").to_ascii_lowercase();
    if let Some(stripped) = out.strip_prefix('+') {
        out = stripped.to_string();
    }
    out
}

pub(super) fn split_dotted_key_preserving_quotes(raw: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_double = false;
    let mut in_single = false;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if in_double {
            if ch == '\\' {
                escaped = true;
                current.push(ch);
                continue;
            }
            if ch == '"' {
                in_double = false;
            }
            current.push(ch);
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            current.push(ch);
            continue;
        }

        match ch {
            '"' => {
                in_double = true;
                current.push(ch);
            }
            '\'' => {
                in_single = true;
                current.push(ch);
            }
            '.' => {
                let part = normalize_key(&current);
                if !part.is_empty() {
                    parts.push(part);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let tail = normalize_key(&current);
    if !tail.is_empty() {
        parts.push(tail);
    }

    parts
}

fn unescape_basic_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        let Some(next) = chars.next() else {
            out.push('\\');
            break;
        };

        match next {
            'b' => out.push('\u{0008}'),
            't' => out.push('\t'),
            'n' => out.push('\n'),
            'f' => out.push('\u{000C}'),
            'r' => out.push('\r'),
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            'u' => {
                let code: String = chars.by_ref().take(4).collect();
                if code.len() == 4 {
                    if let Ok(v) = u32::from_str_radix(&code, 16) {
                        if let Some(c) = char::from_u32(v) {
                            out.push(c);
                        } else {
                            out.push_str("\\u");
                            out.push_str(&code);
                        }
                    } else {
                        out.push_str("\\u");
                        out.push_str(&code);
                    }
                } else {
                    out.push_str("\\u");
                    out.push_str(&code);
                }
            }
            'U' => {
                let code: String = chars.by_ref().take(8).collect();
                if code.len() == 8 {
                    if let Ok(v) = u32::from_str_radix(&code, 16) {
                        if let Some(c) = char::from_u32(v) {
                            out.push(c);
                        } else {
                            out.push_str("\\U");
                            out.push_str(&code);
                        }
                    } else {
                        out.push_str("\\U");
                        out.push_str(&code);
                    }
                } else {
                    out.push_str("\\U");
                    out.push_str(&code);
                }
            }
            other => {
                out.push('\\');
                out.push(other);
            }
        }
    }

    out
}
