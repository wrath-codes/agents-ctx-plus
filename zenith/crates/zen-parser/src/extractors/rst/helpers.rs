use ast_grep_core::Node;
use std::collections::HashMap;

pub(super) fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_string()
}

pub(super) fn section_title<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    if let Some(title) = node.children().find(|c| c.kind().as_ref() == "title") {
        return title.text().trim().to_string();
    }
    first_line(&node.text())
}

pub(super) fn section_level_from_text(text: &str) -> u8 {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() >= 3 {
        let a = lines[0].trim();
        let b = lines[1].trim();
        let c = lines[2].trim();
        if is_adornment(a) && !b.is_empty() && is_adornment(c) {
            return adornment_rank(c.chars().next().unwrap_or('='));
        }
    }
    if lines.len() >= 2 {
        let title = lines[0].trim();
        let adorn = lines[1].trim();
        if !title.is_empty() && is_adornment(adorn) {
            return adornment_rank(adorn.chars().next().unwrap_or('='));
        }
    }
    100
}

pub(super) fn directive_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    if let Some(name) = node.field("name") {
        let raw = name.text();
        let text = raw.trim().trim_start_matches("::").trim();
        if !text.is_empty() {
            return text.to_string();
        }
    }
    "directive".to_string()
}

pub(super) fn target_name<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    if let Some(name) = node.field("name") {
        let raw = name.text();
        let text = raw
            .trim()
            .trim_start_matches('_')
            .trim_end_matches(':')
            .trim();
        if !text.is_empty() {
            return text.to_string();
        }
    }
    if let Some(link) = node.field("link") {
        let raw = link.text();
        let text = raw.trim();
        if !text.is_empty() {
            return text.to_string();
        }
    }
    "target".to_string()
}

pub(super) fn inline_text<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    node.text().trim().to_string()
}

pub(super) fn normalize_label(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_start_matches('|')
        .trim_end_matches('|')
        .to_string()
}

pub(super) fn directive_parts<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> (Option<String>, usize, usize, usize) {
    let mut args_text = None;
    let mut arg_count = 0usize;
    let mut option_count = 0usize;
    let mut body_lines = 0usize;

    if let Some(body) = node.field("body") {
        for child in body.children() {
            match child.kind().as_ref() {
                "arguments" => {
                    let text = child.text().trim().to_string();
                    arg_count = if text.is_empty() {
                        0
                    } else {
                        text.split_whitespace().count()
                    };
                    if !text.is_empty() {
                        args_text = Some(text);
                    }
                }
                "options" => {
                    option_count = child
                        .children()
                        .filter(|c| c.kind().as_ref() == "field")
                        .count();
                }
                "content" => {
                    body_lines = child
                        .text()
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .count();
                }
                _ => {}
            }
        }
    }

    (args_text, arg_count, option_count, body_lines)
}

pub(super) fn directive_option_pairs<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(body) = node.field("body") else {
        return out;
    };
    for child in body.children() {
        if child.kind().as_ref() != "options" {
            continue;
        }
        for field in child.children() {
            if field.kind().as_ref() != "field" {
                continue;
            }
            let mut key = String::new();
            let mut val = String::new();
            for sub in field.children() {
                match sub.kind().as_ref() {
                    "field_name" => key = inline_text(&sub).trim_matches(':').trim().to_string(),
                    "field_body" => val = inline_text(&sub),
                    _ => {}
                }
            }
            if !key.is_empty() {
                out.insert(key, val);
            }
        }
    }
    out
}

pub(super) fn parse_reference_label(signature: &str) -> String {
    let mut s = signature.trim().to_string();
    s = s.trim_end_matches('_').to_string();
    s = s.trim_start_matches('`').trim_end_matches('`').to_string();
    s.trim().to_string()
}

pub(super) fn parse_footnote_ref_label(signature: &str) -> String {
    normalize_label(
        signature
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']'),
    )
}

pub(super) fn extract_role_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    node.children()
        .find(|c| c.kind().as_ref() == "role")
        .map(|r| r.text().trim().trim_matches(':').to_string())
        .filter(|s| !s.is_empty())
}

pub(super) fn detect_table_blocks(source: &str) -> Vec<(u32, u32, String, usize, usize)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim_end();
        if line.starts_with('+') && line.contains("---") {
            let start = i;
            let mut j = i + 1;
            while j < lines.len() {
                let l = lines[j].trim_end();
                if l.starts_with('+') || l.starts_with('|') || l.is_empty() {
                    j += 1;
                    continue;
                }
                break;
            }
            let end = j.saturating_sub(1);
            let row_count = lines[start..=end]
                .iter()
                .filter(|l| l.trim_start().starts_with('|'))
                .count();
            let col_count = line.matches('+').count().saturating_sub(1);
            out.push((
                start as u32 + 1,
                end as u32 + 1,
                "grid_table".to_string(),
                row_count,
                col_count,
            ));
            i = j;
            continue;
        }

        if line.starts_with('=') && line.ends_with('=') && line.contains("==") {
            let start = i;
            let mut j = i + 1;
            while j < lines.len() && !lines[j].trim().is_empty() {
                j += 1;
            }
            if j > start + 1 {
                let end = j - 1;
                let row_count = end.saturating_sub(start + 1);
                let col_count = line.split_whitespace().count().max(1);
                out.push((
                    start as u32 + 1,
                    end as u32 + 1,
                    "simple_table".to_string(),
                    row_count,
                    col_count,
                ));
            }
            i = j + 1;
            continue;
        }
        i += 1;
    }
    out
}

fn is_adornment(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_alphanumeric() {
        return false;
    }
    chars.all(|c| c == first)
}

fn adornment_rank(ch: char) -> u8 {
    match ch {
        '=' => 1,
        '-' => 2,
        '~' => 3,
        '^' => 4,
        '"' => 5,
        '#' => 6,
        '*' => 7,
        '+' => 8,
        '`' => 9,
        _ => 100,
    }
}
