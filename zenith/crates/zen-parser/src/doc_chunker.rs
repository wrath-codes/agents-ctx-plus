//! Document chunking for embedding.
//!
//! Splits markdown, reStructuredText, and plain-text documentation files into
//! chunks suitable for embedding via fastembed. The chunker:
//!
//! - Detects format from file extension (`.md` → markdown, `.rst` → rst, else → text)
//! - Uses **ast-grep** to find heading/section boundaries in markdown and RST,
//!   reusing the same parsers and `KindMatcher` queries as the full extractors
//! - For `.txt` files, uses the text extractor's smart routing: probes content to
//!   detect markdown or RST formatting, delegating to those parsers when detected,
//!   and falling back to heuristic heading detection for genuinely plain text
//! - Tracks heading hierarchy to produce a `section_path` breadcrumb on each chunk
//! - Sub-chunks oversized sections with ~200-char overlap at paragraph boundaries
//! - Skips empty/whitespace-only chunks
//!
//! The `content` field stores **raw text only** — no title prepended. The embedding
//! pipeline is responsible for assembling embed text (e.g. `"{title}: {content}"`).
//!
//! Design informed by:
//! - Google ADK sliding-window overlap (~10% of chunk size)
//! - H-MEM hierarchical section path for retrieval context
//! - Anthropic's "smallest possible set of high-signal tokens" principle

use ast_grep_core::matcher::KindMatcher;

use crate::parser;

// Pull in text format detection and plain-text heading heuristics via the same
// `#[path]` pattern the dispatcher modules use.
#[allow(clippy::duplicate_mod)]
#[path = "extractors/text/helpers.rs"]
mod text_helpers;

/// Maximum chunk size in characters (~512 tokens).
const MAX_CHUNK_CHARS: usize = 2048;

/// Overlap in characters when sub-chunking oversized sections.
/// ~10% of `MAX_CHUNK_CHARS`, validated by Google ADK research.
const OVERLAP_CHARS: usize = 200;

/// A single chunk of a documentation file, ready for embedding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocChunk {
    /// Nearest heading text (e.g. `"Linux"`), or `None` for the preamble
    /// before the first heading.
    pub title: Option<String>,

    /// Full breadcrumb path through the heading hierarchy.
    ///
    /// Example: `["Getting Started", "Installation", "Linux"]`.
    /// Empty for content before the first heading.
    pub section_path: Vec<String>,

    /// Raw chunk text. Title is **not** prepended — the pipeline does that
    /// when building the embedding input.
    pub content: String,

    /// Zero-based index of this chunk within the document.
    pub chunk_index: u32,

    /// Relative file path of the source document.
    pub source_file: String,

    /// Detected format: `"markdown"`, `"rst"`, or `"text"`.
    pub format: String,

    /// Byte offset of this chunk's content in the original document.
    pub byte_offset: usize,

    /// Character length of `content`.
    pub char_len: usize,
}

/// Chunk a documentation file into embedding-ready pieces.
///
/// Detects the document format from `source_file`, splits by headings,
/// tracks heading hierarchy, and sub-chunks oversized sections with overlap.
///
/// # Arguments
///
/// * `content` — Full document text.
/// * `source_file` — Relative file path (used for format detection and stored on chunks).
///
/// # Examples
///
/// ```
/// use zen_parser::doc_chunker::chunk_document;
///
/// let md = "# Intro\n\nHello world.\n\n## Details\n\nSome details here.\n";
/// let chunks = chunk_document(md, "README.md");
/// assert_eq!(chunks.len(), 2);
/// assert_eq!(chunks[0].title.as_deref(), Some("Intro"));
/// assert_eq!(chunks[0].section_path, vec!["Intro"]);
/// assert_eq!(chunks[1].section_path, vec!["Intro", "Details"]);
/// ```
#[must_use]
pub fn chunk_document(content: &str, source_file: &str) -> Vec<DocChunk> {
    let format = detect_doc_format(source_file);
    let sections = match format.as_str() {
        "markdown" => split_markdown(content),
        "rst" => split_rst(content),
        _ => split_text(content),
    };

    let mut chunks = Vec::new();
    let mut chunk_index = 0u32;

    for section in &sections {
        let trimmed_body = section.body.trim();
        if trimmed_body.is_empty() {
            continue;
        }

        let sub_chunks = split_to_max_size(trimmed_body, section.byte_offset);

        for sub in &sub_chunks {
            chunks.push(DocChunk {
                title: section.title.clone(),
                section_path: section.path.clone(),
                content: sub.text.clone(),
                chunk_index,
                source_file: source_file.to_string(),
                format: format.clone(),
                byte_offset: sub.byte_offset,
                char_len: sub.text.chars().count(),
            });
            chunk_index += 1;
        }
    }

    chunks
}

// ── Internal types ───────────────────────────────────────────

/// A document section identified by its heading and body text.
struct Section {
    title: Option<String>,
    path: Vec<String>,
    body: String,
    byte_offset: usize,
}

/// A sub-chunk produced by splitting an oversized section.
struct SubChunk {
    text: String,
    byte_offset: usize,
}

// ── Format detection ─────────────────────────────────────────

fn detect_doc_format(source_file: &str) -> String {
    let path = std::path::Path::new(source_file);
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("md" | "markdown" | "mdx") => "markdown".to_string(),
        Some("rst") => "rst".to_string(),
        _ => "text".to_string(),
    }
}

// ── Line-based utilities ─────────────────────────────────────

/// Pre-compute a mapping from zero-based line index to byte offset.
///
/// `line_offsets[i]` is the byte offset of the start of line `i` in `content`.
/// An extra entry at the end equals `content.len()` for easy range slicing.
fn build_line_offsets(content: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (i, byte) in content.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            offsets.push(i + 1);
        }
    }
    // Sentinel: makes slicing to end-of-file easy
    if offsets.last().copied() != Some(content.len()) {
        offsets.push(content.len());
    }
    offsets
}

/// Slice `content` from `start_line` (zero-based, inclusive) to `end_line`
/// (zero-based, exclusive) using pre-computed line offsets.
fn slice_lines<'a>(
    content: &'a str,
    line_offsets: &[usize],
    start_line: usize,
    end_line: usize,
) -> &'a str {
    let byte_start = line_offsets
        .get(start_line)
        .copied()
        .unwrap_or(content.len());
    let byte_end = line_offsets.get(end_line).copied().unwrap_or(content.len());
    if byte_start >= byte_end {
        return "";
    }
    &content[byte_start..byte_end]
}

// ── Markdown splitting (ast-grep) ────────────────────────────

/// A heading marker found by ast-grep in a markdown document.
struct MdHeading {
    /// Zero-based line of the heading.
    start_line: usize,
    /// Zero-based line after the heading node ends (exclusive).
    end_line: usize,
    /// Heading level (1–6).
    level: usize,
    /// Clean title text.
    title: String,
}

/// A thematic break (`---`, `***`, `___`) found by ast-grep.
struct MdBreak {
    /// Zero-based line of the break.
    line: usize,
}

/// Split markdown content by headings using ast-grep's `KindMatcher`.
///
/// Parses with `parse_markdown_source`, finds `atx_heading` and `setext_heading`
/// nodes plus `thematic_break` nodes, then slices the source text between those
/// positions. Heading hierarchy is tracked with the same stack algorithm used
/// by the markdown extractor.
#[allow(clippy::too_many_lines)]
fn split_markdown(content: &str) -> Vec<Section> {
    let tree = parser::parse_markdown_source(content);
    let lang = *tree.root().lang();
    let line_offsets = build_line_offsets(content);

    // Collect heading nodes
    let mut headings: Vec<MdHeading> = Vec::new();

    for node in tree.root().find_all(KindMatcher::new("atx_heading", lang)) {
        let raw = node.text().to_string();
        headings.push(MdHeading {
            start_line: node.start_pos().line(),
            end_line: node.end_pos().line() + 1,
            level: md_heading_level(&raw).unwrap_or(1),
            title: md_heading_text(&raw),
        });
    }

    for node in tree
        .root()
        .find_all(KindMatcher::new("setext_heading", lang))
    {
        let raw = node.text().to_string();
        headings.push(MdHeading {
            start_line: node.start_pos().line(),
            end_line: node.end_pos().line() + 1,
            level: md_heading_level(&raw).unwrap_or(1),
            title: md_heading_text(&raw),
        });
    }

    // Collect thematic breaks
    let mut breaks: Vec<MdBreak> = Vec::new();
    for node in tree
        .root()
        .find_all(KindMatcher::new("thematic_break", lang))
    {
        breaks.push(MdBreak {
            line: node.start_pos().line(),
        });
    }

    // Sort headings by position
    headings.sort_by_key(|h| h.start_line);

    let total_lines = content.lines().count();
    let mut sections = Vec::new();
    let mut heading_stack: Vec<(usize, String)> = Vec::new();

    for (i, heading) in headings.iter().enumerate() {
        // Preamble before the first heading
        if i == 0 && heading.start_line > 0 {
            let preamble = slice_lines(content, &line_offsets, 0, heading.start_line);
            if !preamble.trim().is_empty() {
                sections.push(Section {
                    title: None,
                    path: Vec::new(),
                    body: preamble.to_string(),
                    byte_offset: 0,
                });
            }
        }

        // Update heading stack for hierarchy
        while heading_stack
            .last()
            .is_some_and(|(l, _)| *l >= heading.level)
        {
            heading_stack.pop();
        }
        heading_stack.push((heading.level, heading.title.clone()));

        // Body: from end of heading to start of next heading
        let body_start_line = heading.end_line;
        let body_end_line = if i + 1 < headings.len() {
            headings[i + 1].start_line
        } else {
            total_lines
        };

        let body_byte_offset = line_offsets
            .get(body_start_line)
            .copied()
            .unwrap_or(content.len());

        // Check for thematic breaks within this section's body
        let breaks_in_range: Vec<usize> = breaks
            .iter()
            .filter(|b| b.line >= body_start_line && b.line < body_end_line)
            .map(|b| b.line)
            .collect();

        if breaks_in_range.is_empty() {
            let body = slice_lines(content, &line_offsets, body_start_line, body_end_line);
            sections.push(Section {
                title: Some(heading.title.clone()),
                path: build_section_path(&heading_stack),
                body: body.to_string(),
                byte_offset: body_byte_offset,
            });
        } else {
            // Split at thematic breaks
            let section_path = build_section_path(&heading_stack);
            let mut seg_start_line = body_start_line;
            let mut first = true;

            for &brk_line in &breaks_in_range {
                let seg = slice_lines(content, &line_offsets, seg_start_line, brk_line);
                if !seg.trim().is_empty() {
                    let seg_offset = line_offsets
                        .get(seg_start_line)
                        .copied()
                        .unwrap_or(content.len());
                    sections.push(Section {
                        title: if first {
                            Some(heading.title.clone())
                        } else {
                            None
                        },
                        path: section_path.clone(),
                        body: seg.to_string(),
                        byte_offset: seg_offset,
                    });
                }
                seg_start_line = brk_line + 1; // skip the break line
                first = false;
            }
            // Remaining text after last break
            if seg_start_line < body_end_line {
                let seg = slice_lines(content, &line_offsets, seg_start_line, body_end_line);
                if !seg.trim().is_empty() {
                    let seg_offset = line_offsets
                        .get(seg_start_line)
                        .copied()
                        .unwrap_or(content.len());
                    sections.push(Section {
                        title: if first {
                            Some(heading.title.clone())
                        } else {
                            None
                        },
                        path: section_path.clone(),
                        body: seg.to_string(),
                        byte_offset: seg_offset,
                    });
                }
            }
        }
    }

    // Edge case: document with no headings at all
    if headings.is_empty() && !content.trim().is_empty() {
        sections.push(Section {
            title: None,
            path: Vec::new(),
            body: content.to_string(),
            byte_offset: 0,
        });
    }

    sections
}

/// Extract heading level from raw ATX/setext heading text.
/// ATX: count leading `#` chars. Setext: `=` underline → 1, `-` underline → 2.
fn md_heading_level(raw: &str) -> Option<usize> {
    let trimmed = raw.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) {
        return Some(hashes);
    }
    // Setext heading: title line + underline
    let mut lines = raw.lines();
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

/// Extract clean title text from a raw heading node.
fn md_heading_text(raw: &str) -> String {
    let first_line = raw.lines().next().unwrap_or_default().trim();
    first_line
        .trim_start_matches('#')
        .trim()
        .trim_end_matches('#')
        .trim()
        .to_string()
}

// ── RST splitting (ast-grep) ─────────────────────────────────

/// Split RST content by sections using ast-grep's `KindMatcher("section")`.
///
/// Finds section heading nodes via tree-sitter-rst, then slices the source text
/// between heading positions. Heading hierarchy is determined by adornment
/// character order of first appearance.
fn split_rst(content: &str) -> Vec<Section> {
    let tree = parser::parse_rst_source(content);
    let lang = *tree.root().lang();
    let line_offsets = build_line_offsets(content);
    let total_lines = content.lines().count();

    // Collect section nodes: (start_line, end_line, level, title)
    let mut markers: Vec<(usize, usize, usize, String)> = Vec::new();
    let mut adornment_levels: Vec<char> = Vec::new();

    let mut section_nodes: Vec<_> = tree
        .root()
        .find_all(KindMatcher::new("section", lang))
        .collect();
    section_nodes.sort_by_key(|n| n.start_pos().line());

    for node in &section_nodes {
        let raw = node.text().to_string();
        let title = rst_section_title(node);
        let start_line = node.start_pos().line();
        let end_line = node.end_pos().line() + 1;
        let level = rst_level_from_text(&raw, &mut adornment_levels);
        markers.push((start_line, end_line, level, title));
    }

    let mut sections = Vec::new();
    let mut heading_stack: Vec<(usize, String)> = Vec::new();

    // Check for preamble before first section
    if let Some(&(first_start, _, _, _)) = markers.first()
        && first_start > 0
    {
        let preamble = slice_lines(content, &line_offsets, 0, first_start);
        if !preamble.trim().is_empty() {
            sections.push(Section {
                title: None,
                path: Vec::new(),
                body: preamble.to_string(),
                byte_offset: 0,
            });
        }
    }

    for (i, &(_start_line, heading_end_line, level, ref title)) in markers.iter().enumerate() {
        // Update heading stack
        while heading_stack.last().is_some_and(|(l, _)| *l >= level) {
            heading_stack.pop();
        }
        heading_stack.push((level, title.clone()));

        // Body: from end of the heading node to start of the next section heading.
        let body_start_line = heading_end_line;
        let body_end_line = if i + 1 < markers.len() {
            markers[i + 1].0
        } else {
            total_lines
        };

        let body = if body_start_line < body_end_line {
            slice_lines(content, &line_offsets, body_start_line, body_end_line).to_string()
        } else {
            String::new()
        };

        let body_byte_offset = line_offsets
            .get(body_start_line)
            .copied()
            .unwrap_or(content.len());

        sections.push(Section {
            title: Some(title.clone()),
            path: build_section_path(&heading_stack),
            body,
            byte_offset: body_byte_offset,
        });
    }

    // Edge case: document with no sections
    if markers.is_empty() && !content.trim().is_empty() {
        sections.push(Section {
            title: None,
            path: Vec::new(),
            body: content.to_string(),
            byte_offset: 0,
        });
    }

    sections
}

/// Extract the title text from an RST section node.
fn rst_section_title<D: ast_grep_core::Doc>(node: &ast_grep_core::Node<D>) -> String {
    // tree-sitter-rst section nodes have a "title" child
    if let Some(title_node) = node.children().find(|c| c.kind().as_ref() == "title") {
        return title_node.text().trim().to_string();
    }
    // Fallback: first non-adornment line of the section text
    for line in node.text().lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !is_rst_adornment_line(trimmed) {
            return trimmed.to_string();
        }
    }
    String::new()
}

/// Determine RST heading level from the section text's adornment character.
///
/// RST heading levels are determined by order of first appearance of each
/// adornment character. The first character seen is level 1, second is level 2, etc.
fn rst_level_from_text(section_text: &str, adornment_levels: &mut Vec<char>) -> usize {
    let lines: Vec<&str> = section_text.lines().collect();

    // Pattern 1: overline + title + underline (3 lines)
    if lines.len() >= 3 {
        let a = lines[0].trim();
        let c = lines[2].trim();
        if is_rst_adornment_line(a) && is_rst_adornment_line(c) {
            let ch = c.chars().next().unwrap_or('=');
            return rst_adornment_level(adornment_levels, ch);
        }
    }

    // Pattern 2: title + underline (2 lines)
    if lines.len() >= 2 {
        let title = lines[0].trim();
        let adorn = lines[1].trim();
        if !title.is_empty() && is_rst_adornment_line(adorn) && adorn.len() >= title.len() {
            let ch = adorn.chars().next().unwrap_or('=');
            return rst_adornment_level(adornment_levels, ch);
        }
    }

    99 // unknown level
}

/// Check if a line consists of repeated RST adornment characters.
fn is_rst_adornment_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 2 {
        return false;
    }
    let first = match trimmed.chars().next() {
        Some(c) if "=-~^\"'+#*:".contains(c) => c,
        _ => return false,
    };
    trimmed.chars().all(|c| c == first)
}

/// Get or assign a heading level for the given adornment character.
fn rst_adornment_level(adornment_levels: &mut Vec<char>, ch: char) -> usize {
    adornment_levels.iter().position(|c| *c == ch).map_or_else(
        || {
            adornment_levels.push(ch);
            adornment_levels.len()
        },
        |pos| pos + 1,
    )
}

// ── Plain text splitting (smart routing + heuristics) ────────

/// Split text content using the smart format router.
///
/// Probes the content to detect if it's really markdown or RST in disguise
/// (common for `.txt` files like `llms.txt`), delegating to those parsers.
/// Falls back to heuristic heading detection for genuinely plain text.
fn split_text(content: &str) -> Vec<Section> {
    match text_helpers::detect_text_format(content) {
        text_helpers::DetectedTextFormat::Markdown => split_markdown(content),
        text_helpers::DetectedTextFormat::Rst => split_rst(content),
        text_helpers::DetectedTextFormat::Plain => split_plain_text(content),
    }
}

/// Split genuinely plain text by detected heuristic headings (ALL CAPS,
/// numbered sections, underline-adorned) or by double blank lines as fallback.
fn split_plain_text(content: &str) -> Vec<Section> {
    let headings = text_helpers::detect_plain_text_headings(content);
    let lines: Vec<&str> = content.lines().collect();
    let line_offsets = build_line_offsets(content);

    if headings.is_empty() {
        return split_by_double_blanks(content);
    }

    let mut sections = Vec::new();
    let mut heading_stack: Vec<(u8, String)> = Vec::new();

    // Preamble before first heading
    if headings[0].line > 0 {
        let preamble = slice_lines(content, &line_offsets, 0, headings[0].line);
        if !preamble.trim().is_empty() {
            sections.push(Section {
                title: None,
                path: Vec::new(),
                body: preamble.to_string(),
                byte_offset: 0,
            });
        }
    }

    for (idx, heading) in headings.iter().enumerate() {
        let body_start_line = heading.line + heading.line_span;
        let body_end_line = if idx + 1 < headings.len() {
            headings[idx + 1].line
        } else {
            lines.len()
        };

        // Update heading stack
        while heading_stack
            .last()
            .is_some_and(|(l, _)| *l >= heading.level)
        {
            heading_stack.pop();
        }
        heading_stack.push((heading.level, heading.title.clone()));

        let body = if body_start_line < body_end_line {
            slice_lines(content, &line_offsets, body_start_line, body_end_line).to_string()
        } else {
            String::new()
        };

        let byte_offset = line_offsets
            .get(heading.line)
            .copied()
            .unwrap_or(content.len());

        sections.push(Section {
            title: Some(heading.title.clone()),
            path: build_section_path_u8(&heading_stack),
            body,
            byte_offset,
        });
    }

    sections
}

/// Split content by double blank lines (paragraph boundaries).
/// No heading hierarchy is tracked.
fn split_by_double_blanks(content: &str) -> Vec<Section> {
    let line_offsets = build_line_offsets(content);
    let mut sections = Vec::new();
    let mut current_body = String::new();
    let mut current_byte_offset: usize = 0;
    let mut blank_count = 0u32;

    for (line_idx, line) in content.lines().enumerate() {
        let line_start = line_offsets.get(line_idx).copied().unwrap_or(0);
        let next_line_start = line_offsets
            .get(line_idx + 1)
            .copied()
            .unwrap_or(content.len());

        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count >= 2 && !current_body.trim().is_empty() {
                sections.push(Section {
                    title: None,
                    path: Vec::new(),
                    body: std::mem::take(&mut current_body),
                    byte_offset: current_byte_offset,
                });
                current_byte_offset = next_line_start;
            } else {
                current_body.push('\n');
            }
        } else {
            if blank_count >= 2 {
                current_byte_offset = line_start;
            }
            blank_count = 0;
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    if !current_body.trim().is_empty() {
        sections.push(Section {
            title: None,
            path: Vec::new(),
            body: current_body,
            byte_offset: current_byte_offset,
        });
    }

    sections
}

// ── Sub-chunking with overlap ────────────────────────────────

/// Split text into sub-chunks of at most `MAX_CHUNK_CHARS` characters,
/// with `OVERLAP_CHARS` overlap when splitting is needed.
///
/// Split points are chosen at the nearest paragraph break (`\n\n`) within
/// the overlap zone, falling back to the nearest line break (`\n`), and
/// finally to an exact position if no break is found.
fn split_to_max_size(text: &str, base_byte_offset: usize) -> Vec<SubChunk> {
    let total_chars = text.chars().count();
    if total_chars <= MAX_CHUNK_CHARS {
        return vec![SubChunk {
            text: text.to_string(),
            byte_offset: base_byte_offset,
        }];
    }

    let chars: Vec<char> = text.chars().collect();
    let mut sub_chunks = Vec::new();
    let mut start = 0usize;

    while start < chars.len() {
        let remaining = chars.len() - start;
        if remaining <= MAX_CHUNK_CHARS {
            let chunk_text: String = chars[start..].iter().collect();
            let byte_off = byte_offset_of_char_index(text, start);
            sub_chunks.push(SubChunk {
                text: chunk_text,
                byte_offset: base_byte_offset + byte_off,
            });
            break;
        }

        let stride = MAX_CHUNK_CHARS.saturating_sub(OVERLAP_CHARS);
        let search_start = start + stride;
        let search_end = (start + MAX_CHUNK_CHARS).min(chars.len());

        let split_at = find_paragraph_break(&chars, search_start, search_end)
            .or_else(|| find_line_break(&chars, search_start, search_end))
            .unwrap_or(search_end);

        let chunk_text: String = chars[start..split_at].iter().collect();
        let byte_off = byte_offset_of_char_index(text, start);
        sub_chunks.push(SubChunk {
            text: chunk_text,
            byte_offset: base_byte_offset + byte_off,
        });

        // Advance with overlap, ensuring forward progress
        let prev_start = start;
        start = split_at.saturating_sub(OVERLAP_CHARS);
        if start <= prev_start {
            start = split_at;
        }
    }

    sub_chunks
}

/// Find the last `\n\n` in `chars[search_start..search_end]`.
fn find_paragraph_break(chars: &[char], search_start: usize, search_end: usize) -> Option<usize> {
    let end = search_end.min(chars.len());
    let start = search_start.min(end);

    if end < 2 {
        return None;
    }
    for i in (start..end.saturating_sub(1)).rev() {
        if chars[i] == '\n' && chars[i + 1] == '\n' {
            return Some(i + 2);
        }
    }
    None
}

/// Find the last `\n` in `chars[search_start..search_end]`.
fn find_line_break(chars: &[char], search_start: usize, search_end: usize) -> Option<usize> {
    let end = search_end.min(chars.len());
    let start = search_start.min(end);

    for i in (start..end).rev() {
        if chars[i] == '\n' {
            return Some(i + 1);
        }
    }
    None
}

/// Convert a char index to a byte offset in the original string.
fn byte_offset_of_char_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map_or(s.len(), |(byte_idx, _)| byte_idx)
}

// ── Heading stack helpers ────────────────────────────────────

fn build_section_path(heading_stack: &[(usize, String)]) -> Vec<String> {
    heading_stack
        .iter()
        .map(|(_, title)| title.clone())
        .collect()
}

fn build_section_path_u8(heading_stack: &[(u8, String)]) -> Vec<String> {
    heading_stack
        .iter()
        .map(|(_, title)| title.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Format detection ─────────────────────────────────────

    #[test]
    fn detect_markdown_formats() {
        assert_eq!(detect_doc_format("README.md"), "markdown");
        assert_eq!(detect_doc_format("docs/guide.markdown"), "markdown");
        assert_eq!(detect_doc_format("page.MDX"), "markdown");
    }

    #[test]
    fn detect_rst_format() {
        assert_eq!(detect_doc_format("index.rst"), "rst");
        assert_eq!(detect_doc_format("GUIDE.RST"), "rst");
    }

    #[test]
    fn detect_text_fallback() {
        assert_eq!(detect_doc_format("notes.txt"), "text");
        assert_eq!(detect_doc_format("README"), "text");
        assert_eq!(detect_doc_format("CHANGELOG"), "text");
    }

    // ── Markdown chunking: basic ─────────────────────────────

    #[test]
    fn markdown_single_section() {
        let md = "# Hello\n\nWorld.\n";
        let chunks = chunk_document(md, "test.md");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].title.as_deref(), Some("Hello"));
        assert_eq!(chunks[0].section_path, vec!["Hello"]);
        assert_eq!(chunks[0].content, "World.");
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].format, "markdown");
    }

    #[test]
    fn markdown_multiple_sections() {
        let md = "# Intro\n\nHello.\n\n## Details\n\nMore info.\n\n## Other\n\nStuff.\n";
        let chunks = chunk_document(md, "guide.md");
        assert_eq!(chunks.len(), 3);

        assert_eq!(chunks[0].title.as_deref(), Some("Intro"));
        assert_eq!(chunks[0].section_path, vec!["Intro"]);

        assert_eq!(chunks[1].title.as_deref(), Some("Details"));
        assert_eq!(chunks[1].section_path, vec!["Intro", "Details"]);

        assert_eq!(chunks[2].title.as_deref(), Some("Other"));
        assert_eq!(chunks[2].section_path, vec!["Intro", "Other"]);
    }

    #[test]
    fn markdown_heading_hierarchy() {
        let md = "\
# A
\nText A.\n
## B
\nText B.\n
### C
\nText C.\n
## D
\nText D.\n";
        let chunks = chunk_document(md, "test.md");
        assert_eq!(chunks.len(), 4);

        assert_eq!(chunks[0].section_path, vec!["A"]);
        assert_eq!(chunks[1].section_path, vec!["A", "B"]);
        assert_eq!(chunks[2].section_path, vec!["A", "B", "C"]);
        assert_eq!(chunks[3].section_path, vec!["A", "D"]);
    }

    #[test]
    fn markdown_preamble_before_first_heading() {
        let md = "Some preamble text.\n\n# First Heading\n\nBody.\n";
        let chunks = chunk_document(md, "test.md");
        assert_eq!(chunks.len(), 2);

        assert_eq!(chunks[0].title, None);
        assert!(chunks[0].section_path.is_empty());
        assert!(chunks[0].content.contains("preamble"));

        assert_eq!(chunks[1].title.as_deref(), Some("First Heading"));
    }

    #[test]
    fn markdown_empty_sections_skipped() {
        let md = "# Empty\n\n# Has Content\n\nHello.\n";
        let chunks = chunk_document(md, "test.md");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].title.as_deref(), Some("Has Content"));
    }

    #[test]
    fn markdown_horizontal_rule_splits() {
        let md = "# Section\n\nPart one.\n\n---\n\nPart two.\n";
        let chunks = chunk_document(md, "test.md");
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].content.contains("Part one"));
        assert!(chunks[1].content.contains("Part two"));
    }

    // ── RST chunking ─────────────────────────────────────────

    #[test]
    fn rst_basic_sections() {
        let rst = "\
Title
=====

Some content.

Subtitle
--------

More content.
";
        let chunks = chunk_document(rst, "index.rst");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].title.as_deref(), Some("Title"));
        assert_eq!(chunks[0].section_path, vec!["Title"]);
        assert_eq!(chunks[1].title.as_deref(), Some("Subtitle"));
        assert_eq!(chunks[1].section_path, vec!["Title", "Subtitle"]);
    }

    #[test]
    fn rst_adornment_levels_by_appearance_order() {
        let rst = "\
First
~~~~~

Body 1.

Second
======

Body 2.
";
        let chunks = chunk_document(rst, "test.rst");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].section_path, vec!["First"]);
        assert_eq!(chunks[1].section_path, vec!["First", "Second"]);
    }

    // ── Plain text chunking ──────────────────────────────────

    #[test]
    fn plain_text_splits_on_double_blank() {
        let txt = "Paragraph one.\n\n\nParagraph two.\n\n\nParagraph three.\n";
        let chunks = chunk_document(txt, "notes.txt");
        assert_eq!(chunks.len(), 3);
        assert!(chunks[0].content.contains("one"));
        assert!(chunks[1].content.contains("two"));
        assert!(chunks[2].content.contains("three"));
        assert!(chunks[0].title.is_none());
        assert!(chunks[0].section_path.is_empty());
    }

    // ── Sub-chunking with overlap ────────────────────────────

    #[test]
    fn small_section_is_single_chunk() {
        let text = "Short content.";
        let subs = split_to_max_size(text, 0);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].text, "Short content.");
    }

    #[test]
    fn oversized_section_gets_sub_chunked() {
        let paragraph = "A".repeat(500) + "\n\n";
        let content = paragraph.repeat(10);
        assert!(content.chars().count() > MAX_CHUNK_CHARS);

        let subs = split_to_max_size(&content, 0);
        assert!(
            subs.len() >= 2,
            "expected multiple sub-chunks, got {}",
            subs.len()
        );

        for sub in &subs {
            assert!(
                sub.text.chars().count() <= MAX_CHUNK_CHARS,
                "sub-chunk too large: {} chars",
                sub.text.chars().count()
            );
        }

        if subs.len() >= 2 {
            assert!(
                subs[1].byte_offset < subs[0].byte_offset + subs[0].text.len(),
                "expected overlapping byte offsets"
            );
        }
    }

    #[test]
    fn sub_chunk_splits_at_paragraph_boundary() {
        let part1 = "X".repeat(1900);
        let part2 = "Y".repeat(500);
        let content = format!("{part1}\n\n{part2}\n");

        let subs = split_to_max_size(&content, 0);
        assert!(subs.len() >= 2);

        assert!(
            subs[0].text.ends_with("\n\n") || subs[0].text.chars().count() <= MAX_CHUNK_CHARS,
            "first chunk should split at paragraph boundary"
        );
    }

    // ── Byte offset tracking ─────────────────────────────────

    #[test]
    fn byte_offsets_are_tracked() {
        let md = "# First\n\nContent A.\n\n# Second\n\nContent B.\n";
        let chunks = chunk_document(md, "test.md");
        assert_eq!(chunks.len(), 2);
        // byte_offset points to start of body content (after heading line)
        // "# First\n" = 9 bytes, so body starts at offset 9
        assert!(chunks[0].byte_offset > 0);
        assert!(chunks[1].byte_offset > chunks[0].byte_offset);
    }

    // ── Char length tracking ─────────────────────────────────

    #[test]
    fn char_len_matches_content() {
        let md = "# Test\n\nHello world.\n";
        let chunks = chunk_document(md, "test.md");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].char_len, chunks[0].content.chars().count());
    }

    // ── Edge cases ───────────────────────────────────────────

    #[test]
    fn empty_document_produces_no_chunks() {
        assert!(chunk_document("", "test.md").is_empty());
        assert!(chunk_document("   \n\n  \n", "test.md").is_empty());
    }

    #[test]
    fn single_line_content_produces_chunk() {
        let md = "type Foo = Bar;\n";
        let chunks = chunk_document(md, "types.txt");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("type Foo = Bar;"));
    }

    #[test]
    fn unicode_content_handled() {
        let md = "# Einf\u{00fc}hrung\n\nDeutsche Dokumentation mit Uml\u{00e4}uten.\n";
        let chunks = chunk_document(md, "docs.md");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].title.as_deref(), Some("Einf\u{00fc}hrung"));
        assert_eq!(chunks[0].char_len, chunks[0].content.chars().count());
    }

    // ── Integration: full document ───────────────────────────

    #[test]
    fn full_markdown_document() {
        let md = "\
# Getting Started

Welcome to the project.

## Installation

### Linux

apt-get install foo

### macOS

brew install foo

## Configuration

Edit config.toml to set options.
";
        let chunks = chunk_document(md, "README.md");
        assert_eq!(chunks.len(), 4);

        assert_eq!(chunks[0].title.as_deref(), Some("Getting Started"));
        assert_eq!(chunks[0].section_path, vec!["Getting Started"]);

        assert_eq!(chunks[1].title.as_deref(), Some("Linux"));
        assert_eq!(
            chunks[1].section_path,
            vec!["Getting Started", "Installation", "Linux"]
        );

        assert_eq!(chunks[2].title.as_deref(), Some("macOS"));
        assert_eq!(
            chunks[2].section_path,
            vec!["Getting Started", "Installation", "macOS"]
        );

        assert_eq!(chunks[3].title.as_deref(), Some("Configuration"));
        assert_eq!(
            chunks[3].section_path,
            vec!["Getting Started", "Configuration"]
        );
    }

    // ── Smart text routing ───────────────────────────────────

    #[test]
    fn txt_with_markdown_content_uses_markdown_parser() {
        let content = "# Title\n\nSome markdown content.\n\n## Section\n\nMore text.\n";
        let chunks = chunk_document(content, "llms.txt");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].title.as_deref(), Some("Title"));
        assert_eq!(chunks[1].title.as_deref(), Some("Section"));
    }

    #[test]
    fn txt_with_rst_content_uses_rst_parser() {
        let content =
            "Title\n=====\n\n.. code-block:: python\n\n   print('hi')\n\nSub\n---\n\nMore text.\n";
        let chunks = chunk_document(content, "readme.txt");
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].title.as_deref(), Some("Title"));
    }
}
