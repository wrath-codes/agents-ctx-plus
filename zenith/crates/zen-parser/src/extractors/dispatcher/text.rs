//! Plain-text extractor with smart format routing.
//!
//! Acts as a router for `.txt` files:
//! - If content looks like **Markdown** (ATX headings, blockquotes, link lists
//!   — common in `llms.txt`), delegates to the markdown extractor.
//! - If content looks like **RST** (underline-adorned headings, directives —
//!   common in Sphinx-generated `.txt`), delegates to the RST extractor.
//! - Otherwise, uses **heuristic heading detection** (ALL CAPS headings,
//!   numbered sections, underline headings) for truly plain text.

use crate::types::ParsedItem;

#[path = "../text/helpers.rs"]
mod helpers;
#[path = "../text/processors.rs"]
mod processors;

use helpers::DetectedTextFormat;

/// Extract significant elements from a plain-text document.
///
/// Probes the content to detect Markdown or RST formatting and delegates
/// to the appropriate extractor. Falls back to heuristic heading detection
/// for genuinely unstructured text.
///
/// # Errors
/// Returns `ParserError` if the delegated parser fails.
pub fn extract(source: &str) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    match helpers::detect_text_format(source) {
        DetectedTextFormat::Markdown => {
            let root = crate::parser::parse_markdown_source(source);
            super::markdown::extract(&root)
        }
        DetectedTextFormat::Rst => {
            let root = crate::parser::parse_rst_source(source);
            super::rst::extract(&root)
        }
        DetectedTextFormat::Plain => Ok(extract_plain(source)),
    }
}

/// Heuristic extraction for genuinely plain text.
///
/// Detects headings via ALL CAPS, underline adornment, and numbered patterns.
/// Content between headings is captured as paragraph items.
fn extract_plain(source: &str) -> Vec<ParsedItem> {
    let lines: Vec<&str> = source.lines().collect();
    let total_lines = lines.len() as u32;

    let mut items = vec![processors::root_item(total_lines)];
    let headings = helpers::detect_plain_text_headings(source);

    if headings.is_empty() {
        // No headings found — treat entire document as paragraphs split by double blanks
        let paragraphs = split_paragraphs(&lines);
        for (start, end, first_line) in paragraphs {
            items.push(processors::paragraph_item(
                start as u32 + 1,
                end as u32 + 1,
                first_line,
            ));
        }
        return items;
    }

    // Build section path hierarchy (same stack algorithm as markdown/rst extractors)
    let mut heading_stack: Vec<(u8, String)> = Vec::new();

    for (idx, heading) in headings.iter().enumerate() {
        let end_line = if idx + 1 < headings.len() {
            headings[idx + 1].line as u32
        } else {
            total_lines
        };

        // Update heading stack for hierarchy
        while heading_stack
            .last()
            .is_some_and(|(l, _)| *l >= heading.level)
        {
            heading_stack.pop();
        }

        let mut item = processors::heading_item(heading, end_line);

        // Set owner from parent heading
        if let Some((_, parent_path)) = heading_stack.last() {
            item.metadata.owner_name = Some(parent_path.clone());
            item.metadata.owner_kind = Some(crate::types::SymbolKind::Module);
            item.metadata
                .attributes
                .push(format!("txt:owner_path:{parent_path}"));
        }

        let path = if let Some((_, parent_path)) = heading_stack.last() {
            format!("{parent_path}/{}", heading.title)
        } else {
            heading.title.clone()
        };
        item.metadata.attributes.push(format!("txt:path:{path}"));

        heading_stack.push((heading.level, path));
        items.push(item);
    }

    items
}

/// Split lines into paragraph groups separated by double blank lines.
/// Returns `(start_line_idx, end_line_idx, first_line_text)` for each paragraph.
fn split_paragraphs<'a>(lines: &'a [&'a str]) -> Vec<(usize, usize, &'a str)> {
    let mut paragraphs = Vec::new();
    let mut para_start: Option<usize> = None;
    let mut blank_count = 0u32;

    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count >= 2
                && let Some(start) = para_start.take()
            {
                let end = i.saturating_sub(blank_count as usize);
                if end >= start {
                    paragraphs.push((start, end, lines[start]));
                }
            }
        } else {
            if para_start.is_none() {
                para_start = Some(i);
            }
            blank_count = 0;
        }
    }

    // Flush last paragraph, excluding trailing blank lines
    if let Some(start) = para_start {
        let mut end = lines.len().saturating_sub(1);
        while end > start && lines[end].trim().is_empty() {
            end -= 1;
        }
        paragraphs.push((start, end, lines[start]));
    }

    paragraphs
}

#[cfg(test)]
#[path = "../text/tests/mod.rs"]
mod tests;
