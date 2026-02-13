//! Build `ParsedItem`s from plain-text heuristic headings.

use crate::types::{CommonMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::helpers::PlainTextHeading;

/// Build a `ParsedItem` for a document root element.
pub(super) fn root_item(total_lines: u32) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("txt:kind:document");
    ParsedItem {
        kind: SymbolKind::Module,
        name: "$".to_string(),
        signature: "document".to_string(),
        source: None,
        doc_comment: String::new(),
        start_line: 1,
        end_line: total_lines.max(1),
        visibility: Visibility::Public,
        metadata,
    }
}

/// Build a `ParsedItem` from a heuristic heading.
pub(super) fn heading_item(heading: &PlainTextHeading, end_line: u32) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("txt:kind:heading");
    metadata.push_attribute(format!("txt:level:{}", heading.level));

    ParsedItem {
        kind: SymbolKind::Module,
        name: heading.title.clone(),
        signature: heading.title.clone(),
        source: None,
        doc_comment: String::new(),
        start_line: heading.line as u32 + 1,
        end_line,
        visibility: Visibility::Public,
        metadata,
    }
}

/// Build a `ParsedItem` for a paragraph block (no heading context).
pub(super) fn paragraph_item(start_line: u32, end_line: u32, first_line_text: &str) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("txt:kind:paragraph");

    let name = truncate_name(first_line_text, 60);
    ParsedItem {
        kind: SymbolKind::Property,
        name,
        signature: String::new(),
        source: None,
        doc_comment: String::new(),
        start_line,
        end_line,
        visibility: Visibility::Public,
        metadata,
    }
}

/// Truncate a string to at most `max_len` chars, appending `...` if truncated.
fn truncate_name(s: &str, max_len: usize) -> String {
    let trimmed = s.trim();
    let char_count = trimmed.chars().count();
    if char_count <= max_len {
        trimmed.to_string()
    } else {
        let take = max_len.saturating_sub(3);
        let truncated: String = trimmed.chars().take(take).collect();
        format!("{truncated}...")
    }
}
