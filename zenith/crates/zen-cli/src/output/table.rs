#[derive(Clone, Copy, Debug)]
pub struct TableOptions {
    pub max_width: Option<usize>,
    pub color: bool,
}

/// Render a simple aligned table for string rows.
#[must_use]
pub fn render_entity_table(
    headers: &[&str],
    rows: &[Vec<String>],
    options: TableOptions,
) -> String {
    let mut widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .map(std::string::String::len)
                .max()
                .unwrap_or(0)
                .max(header.len())
                .max(6)
        })
        .collect();

    fit_widths(&mut widths, headers, options.max_width);

    let header_line = headers
        .iter()
        .zip(widths.iter())
        .map(|(header, width)| {
            let text = truncate_text(header, *width);
            format_cell(&text, *width, false, false)
        })
        .collect::<Vec<_>>()
        .join("  ");

    let divider = "-".repeat(strip_ansi(&header_line).len());

    let row_lines = rows
        .iter()
        .map(|row| {
            widths
                .iter()
                .enumerate()
                .map(|(index, width)| {
                    let value = row.get(index).cloned().unwrap_or_else(|| "-".to_string());
                    let truncated = truncate_text(&value, *width);
                    let numeric = looks_numeric(&truncated);
                    let colored = if options.color {
                        colorize_status(&truncated)
                    } else {
                        truncated
                    };
                    format_cell(&colored, *width, numeric, options.color)
                })
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect::<Vec<_>>();

    let mut lines = Vec::with_capacity(2 + row_lines.len());
    lines.push(header_line);
    lines.push(divider);
    lines.extend(row_lines);
    lines.join("\n")
}

fn fit_widths(widths: &mut [usize], headers: &[&str], max_width: Option<usize>) {
    let Some(max_width) = max_width else {
        return;
    };

    if widths.is_empty() {
        return;
    }

    let separators = widths.len().saturating_sub(1) * 2;
    let mut total = widths.iter().sum::<usize>() + separators;
    if total <= max_width {
        return;
    }

    loop {
        if total <= max_width {
            break;
        }

        let mut candidate_idx = None;
        let mut candidate_width = 0usize;
        for (idx, width) in widths.iter().enumerate() {
            let min_width = headers[idx].len().max(6);
            if *width > min_width && *width > candidate_width {
                candidate_idx = Some(idx);
                candidate_width = *width;
            }
        }

        let Some(idx) = candidate_idx else {
            break;
        };

        widths[idx] = widths[idx].saturating_sub(1);
        total = widths.iter().sum::<usize>() + separators;
    }
}

fn truncate_text(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    if width <= 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    for ch in value.chars().take(width - 1) {
        out.push(ch);
    }
    out.push('…');
    out
}

fn looks_numeric(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.' | ','))
}

fn format_cell(value: &str, width: usize, numeric: bool, has_ansi: bool) -> String {
    let plain_len = if has_ansi {
        strip_ansi(value).len()
    } else {
        value.len()
    };
    let pad = width.saturating_sub(plain_len);
    if numeric {
        format!("{}{}", " ".repeat(pad), value)
    } else {
        format!("{}{}", value, " ".repeat(pad))
    }
}

fn colorize_status(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let code = if matches!(
        lower.as_str(),
        "ok" | "true" | "installed" | "authenticated" | "synced" | "pass" | "healthy"
    ) {
        Some("32")
    } else if matches!(
        lower.as_str(),
        "warn" | "warning" | "degraded" | "stale" | "skipped" | "pending"
    ) {
        Some("33")
    } else if matches!(
        lower.as_str(),
        "error" | "failed" | "false" | "missing" | "invalid" | "cancelled"
    ) {
        Some("31")
    } else {
        None
    };

    match code {
        Some(code) => format!("\u{1b}[{code}m{value}\u{1b}[0m"),
        None => value.to_string(),
    }
}

fn strip_ansi(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}
