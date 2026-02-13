pub(super) fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_string()
}

pub(super) fn heading_level(text: &str) -> Option<u8> {
    let trimmed = text.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) {
        return u8::try_from(hashes).ok();
    }

    let mut lines = text.lines();
    let _title = lines.next()?;
    let underline = lines.next()?.trim();
    if underline.starts_with('=') {
        Some(1)
    } else if underline.starts_with('-') {
        Some(2)
    } else {
        None
    }
}

pub(super) fn heading_text(raw: &str) -> String {
    first_line(raw)
        .trim_start_matches('#')
        .trim()
        .trim_end_matches('#')
        .trim()
        .to_string()
}

pub(super) fn code_fence_language(raw: &str) -> Option<String> {
    let line = first_line(raw);
    if !(line.starts_with("```") || line.starts_with("~~~")) {
        return None;
    }

    let suffix = line.get(3..)?.trim();
    if suffix.is_empty() {
        None
    } else {
        Some(suffix.to_string())
    }
}

pub(super) fn list_item_count(raw: &str) -> usize {
    raw.lines()
        .filter(|line| {
            let t = line.trim_start();
            t.starts_with("- ")
                || t.starts_with("* ")
                || t.starts_with("+ ")
                || (t.chars().next().is_some_and(|c| c.is_ascii_digit())
                    && (t.contains(". ") || t.contains(") ")))
        })
        .count()
}

pub(super) fn link_reference_label(raw: &str) -> String {
    raw.split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

pub(super) fn extract_inline_links(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        if i > 0 && bytes[i - 1] == b'!' {
            i += 1;
            continue;
        }

        let Some(close_bracket) = line[i + 1..].find(']') else {
            break;
        };
        let close_bracket = i + 1 + close_bracket;
        if close_bracket + 1 >= bytes.len() || bytes[close_bracket + 1] != b'(' {
            i += 1;
            continue;
        }

        let Some(close_paren_rel) = line[close_bracket + 2..].find(')') else {
            i += 1;
            continue;
        };
        let close_paren = close_bracket + 2 + close_paren_rel;

        let label = line[i + 1..close_bracket].trim().to_string();
        let url = line[close_bracket + 2..close_paren].trim().to_string();
        if !url.is_empty() {
            out.push((label, url));
        }
        i = close_paren + 1;
    }
    out
}

pub(super) fn extract_inline_code(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let Some(close_rel) = line[i + 1..].find('`') else {
            break;
        };
        let close = i + 1 + close_rel;
        let snippet = line[i + 1..close].trim();
        if !snippet.is_empty() {
            out.push(snippet.to_string());
        }
        i = close + 1;
    }

    out
}

pub(super) fn extract_inline_images(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] != b'!' || bytes[i + 1] != b'[' {
            i += 1;
            continue;
        }

        let Some(close_bracket_rel) = line[i + 2..].find(']') else {
            break;
        };
        let close_bracket = i + 2 + close_bracket_rel;
        if close_bracket + 1 >= bytes.len() || bytes[close_bracket + 1] != b'(' {
            i += 1;
            continue;
        }

        let Some(close_paren_rel) = line[close_bracket + 2..].find(')') else {
            i += 1;
            continue;
        };
        let close_paren = close_bracket + 2 + close_paren_rel;

        let alt = line[i + 2..close_bracket].trim().to_string();
        let src = line[close_bracket + 2..close_paren].trim().to_string();
        if !src.is_empty() {
            out.push((alt, src));
        }
        i = close_paren + 1;
    }
    out
}

pub(super) fn extract_autolinks(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let Some(close_rel) = line[i + 1..].find('>') else {
            break;
        };
        let close = i + 1 + close_rel;
        let candidate = line[i + 1..close].trim();
        if candidate.contains("://") || candidate.starts_with("mailto:") {
            out.push(candidate.to_string());
        }
        i = close + 1;
    }

    out
}

pub(super) fn extract_reference_links(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        if i > 0 && bytes[i - 1] == b'!' {
            i += 1;
            continue;
        }

        let Some(label_close_rel) = line[i + 1..].find(']') else {
            break;
        };
        let label_close = i + 1 + label_close_rel;
        if label_close + 1 >= bytes.len() || bytes[label_close + 1] != b'[' {
            i += 1;
            continue;
        }

        let Some(ref_close_rel) = line[label_close + 2..].find(']') else {
            i += 1;
            continue;
        };
        let ref_close = label_close + 2 + ref_close_rel;

        let label = line[i + 1..label_close].trim().to_string();
        let reference = line[label_close + 2..ref_close].trim();
        let reference = if reference.is_empty() {
            label.clone()
        } else {
            reference.to_string()
        };
        if !reference.is_empty() {
            out.push((label, reference));
        }

        i = ref_close + 1;
    }

    out
}

pub(super) fn extract_bare_urls(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < line.len() {
        let slice = &line[i..];
        let rel = slice
            .find("https://")
            .or_else(|| slice.find("http://"))
            .or_else(|| slice.find("mailto:"));
        let Some(rel) = rel else {
            break;
        };
        let start = i + rel;

        if start > 0 {
            let prev = line.as_bytes()[start - 1] as char;
            if prev == '<' || prev == '(' {
                i = start + 1;
                continue;
            }
        }

        let mut end = start;
        for (off, ch) in line[start..].char_indices() {
            if ch.is_whitespace() || ch == ')' || ch == ']' || ch == '>' || ch == '"' || ch == '\''
            {
                break;
            }
            end = start + off + ch.len_utf8();
        }

        if end > start {
            let mut url = line[start..end].to_string();
            while matches!(url.chars().last(), Some('.' | ',' | ';' | ':' | '!' | '?')) {
                url.pop();
            }
            if !url.is_empty() {
                out.push(url);
            }
        }

        i = end.saturating_add(1);
    }

    out
}
