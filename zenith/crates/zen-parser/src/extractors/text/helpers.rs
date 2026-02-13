//! Format detection and heading heuristics for plain-text documents.

/// Detected format of a `.txt` file based on content heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DetectedTextFormat {
    /// Content has Markdown signals (`# `, `## `, `> `, `- [text](url)`).
    Markdown,
    /// Content has RST signals (underline-adorned headings, `.. ` directives, `:role:`).
    Rst,
    /// Truly unstructured text — use heuristic heading detection.
    Plain,
}

/// Detect whether the content of a `.txt` file is Markdown, RST, or plain text.
///
/// Uses a simple scoring system: each signal adds a point to its format.
/// The format with the most signals wins. Tie or zero signals → Plain.
pub(super) fn detect_text_format(content: &str) -> DetectedTextFormat {
    let mut md_score: u32 = 0;
    let mut rst_score: u32 = 0;

    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Markdown signals
        if trimmed.starts_with("# ") || trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            md_score += 2;
        }
        if trimmed.starts_with("> ") {
            md_score += 1;
        }
        if trimmed.starts_with("- [") && trimmed.contains("](") {
            md_score += 1;
        }

        // RST signals
        if trimmed.starts_with(".. ") && trimmed.contains("::") {
            rst_score += 2;
        }
        if trimmed.contains("`:") || trimmed.contains(":`") {
            rst_score += 1;
        }
        // Underline-adorned heading: text line followed by line of repeated adornment chars
        if i + 1 < lines.len() && !trimmed.is_empty() && is_rst_underline(lines[i + 1], trimmed) {
            rst_score += 2;
        }
    }

    if md_score == 0 && rst_score == 0 {
        return DetectedTextFormat::Plain;
    }
    if md_score >= rst_score {
        DetectedTextFormat::Markdown
    } else {
        DetectedTextFormat::Rst
    }
}

/// Check if `underline` is a valid RST underline for `title`.
fn is_rst_underline(underline: &str, title: &str) -> bool {
    let trimmed = underline.trim();
    if trimmed.len() < 2 || trimmed.len() < title.trim().len() {
        return false;
    }
    let first = match trimmed.chars().next() {
        Some(c) if "=-~^\"'+#*:".contains(c) => c,
        _ => return false,
    };
    trimmed.chars().all(|c| c == first)
}

/// A plain-text heading detected by heuristic.
#[derive(Debug, Clone)]
pub(super) struct PlainTextHeading {
    /// The heading text (cleaned up).
    pub title: String,
    /// Inferred heading level (1 = top, 2 = sub, etc.).
    pub level: u8,
    /// Zero-based line index in the source.
    pub line: usize,
    /// Number of source lines consumed by the heading (1 for single-line, 2 for underlined).
    #[allow(dead_code)]
    pub line_span: usize,
}

/// Extract headings from plain text using heuristics.
///
/// Recognizes:
/// - **Underline-adorned headings**: `Title\n=====` or `Title\n-----`
///   (level determined by adornment character, first-seen order)
/// - **ALL CAPS headings**: Lines that are all uppercase, >= 3 chars, not a
///   common constant or abbreviation
/// - **Numbered section headings**: `1. Introduction`, `1.1 Setup`, `A. Appendix`
pub(super) fn detect_plain_text_headings(content: &str) -> Vec<PlainTextHeading> {
    let lines: Vec<&str> = content.lines().collect();
    let mut headings = Vec::new();
    let mut adornment_chars: Vec<char> = Vec::new();
    let mut skip_next = false;

    for (i, line) in lines.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        let trimmed = line.trim();

        // 1. Underline-adorned heading: text line followed by repeated adornment chars
        if i + 1 < lines.len() && !trimmed.is_empty() && is_rst_underline(lines[i + 1], trimmed) {
            let adorn_char = lines[i + 1].trim().chars().next().unwrap_or('=');
            let level = adornment_level(&mut adornment_chars, adorn_char);
            headings.push(PlainTextHeading {
                title: trimmed.to_string(),
                level,
                line: i,
                line_span: 2,
            });
            skip_next = true;
            continue;
        }

        // 2. ALL CAPS heading: line is all uppercase, >= 3 non-whitespace chars,
        //    not a single word that looks like a constant (e.g., "TODO", "NOTE")
        if is_all_caps_heading(trimmed) {
            headings.push(PlainTextHeading {
                title: to_title_case(trimmed),
                level: 1,
                line: i,
                line_span: 1,
            });
            continue;
        }

        // 3. Numbered section heading: `1. Title`, `1.1 Title`, `A. Title`
        if let Some(title) = parse_numbered_heading(trimmed) {
            headings.push(PlainTextHeading {
                title,
                level: 2,
                line: i,
                line_span: 1,
            });
        }
    }

    headings
}

/// Check if a line is an ALL CAPS heading.
///
/// Must be >= 3 chars, all alphabetic chars are uppercase, contains at least
/// one letter, and has at least 2 words (to avoid matching constants like `TODO`).
fn is_all_caps_heading(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let has_letter = trimmed.chars().any(char::is_alphabetic);
    if !has_letter {
        return false;
    }
    let all_upper = trimmed
        .chars()
        .filter(|c| c.is_alphabetic())
        .all(char::is_uppercase);
    if !all_upper {
        return false;
    }
    // Must have at least 2 words to distinguish from constants
    let word_count = trimmed.split_whitespace().count();
    word_count >= 2
}

/// Convert an ALL CAPS string to Title Case.
fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                let rest: String = chars.collect::<String>().to_lowercase();
                format!("{}{rest}", first.to_uppercase().collect::<String>())
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse a numbered heading like `1. Introduction` or `1.1 Setup` or `A. Appendix`.
///
/// Returns the title text without the number prefix, or `None` if the line
/// doesn't match the pattern.
fn parse_numbered_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Pattern: digits (with optional `.` separators) followed by `. ` or ` `
    // e.g., `1. Introduction`, `1.1 Setup`, `2.3.1 Details`
    let mut chars = trimmed.chars().peekable();
    let first = chars.peek()?;

    if first.is_ascii_digit() {
        // Consume digits and dots
        let mut prefix_len = 0;
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                chars.next();
                prefix_len += 1;
            } else {
                break;
            }
        }
        // Must be followed by `. ` or ` `
        let rest = &trimmed[prefix_len..];
        let title = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(' '))?;
        let title = title.trim();
        if !title.is_empty() && title.chars().next()?.is_uppercase() {
            return Some(title.to_string());
        }
    } else if first.is_ascii_uppercase() {
        // Pattern: single uppercase letter followed by `. ` (e.g., `A. Appendix`)
        let letter = chars.next()?;
        if letter.is_ascii_uppercase() {
            let rest = &trimmed[1..];
            if let Some(title) = rest.strip_prefix(". ") {
                let title = title.trim();
                if !title.is_empty() {
                    return Some(title.to_string());
                }
            }
        }
    }

    None
}

fn adornment_level(seen: &mut Vec<char>, ch: char) -> u8 {
    seen.iter().position(|c| *c == ch).map_or_else(
        || {
            seen.push(ch);
            seen.len() as u8
        },
        |pos| (pos + 1) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Format detection ─────────────────────────────────

    #[test]
    fn detects_markdown() {
        let content = "# Title\n\n> Summary\n\n## Section\n\n- [link](url): desc\n";
        assert_eq!(detect_text_format(content), DetectedTextFormat::Markdown);
    }

    #[test]
    fn detects_rst() {
        let content = "Title\n=====\n\n.. code-block:: python\n\n   print('hi')\n\nSub\n---\n";
        assert_eq!(detect_text_format(content), DetectedTextFormat::Rst);
    }

    #[test]
    fn detects_plain_for_unstructured() {
        let content = "Just some plain text.\nWith no structure.\n";
        assert_eq!(detect_text_format(content), DetectedTextFormat::Plain);
    }

    #[test]
    fn markdown_wins_over_rst_when_mixed() {
        let content = "# Title\n\n## Section\n\nSome text\n=====\n";
        // 2 Markdown ATX headings (4 points) vs 1 RST underline (2 points)
        assert_eq!(detect_text_format(content), DetectedTextFormat::Markdown);
    }

    // ── ALL CAPS heading detection ───────────────────────

    #[test]
    fn all_caps_headings() {
        assert!(is_all_caps_heading("GETTING STARTED"));
        assert!(is_all_caps_heading("BASIC LOGGING TUTORIAL"));
        assert!(!is_all_caps_heading("TODO")); // single word
        assert!(!is_all_caps_heading("OK")); // too short
        assert!(!is_all_caps_heading("Not All Caps"));
        assert!(!is_all_caps_heading("123 456")); // no letters
    }

    #[test]
    fn title_case_conversion() {
        assert_eq!(to_title_case("GETTING STARTED"), "Getting Started");
        assert_eq!(to_title_case("HELLO WORLD"), "Hello World");
    }

    // ── Numbered heading detection ───────────────────────

    #[test]
    fn numbered_headings() {
        assert_eq!(
            parse_numbered_heading("1. Introduction"),
            Some("Introduction".to_string())
        );
        assert_eq!(
            parse_numbered_heading("1.1 Setup"),
            Some("Setup".to_string())
        );
        assert_eq!(
            parse_numbered_heading("2.3.1 Details"),
            Some("Details".to_string())
        );
        assert_eq!(
            parse_numbered_heading("A. Appendix"),
            Some("Appendix".to_string())
        );
    }

    #[test]
    fn numbered_heading_rejects_non_headings() {
        // Lowercase after number — not a heading
        assert_eq!(parse_numbered_heading("1. lowercase"), None);
        // No text after number
        assert_eq!(parse_numbered_heading("1."), None);
        // Just a number
        assert_eq!(parse_numbered_heading("42"), None);
    }

    // ── Integrated heading detection ─────────────────────

    #[test]
    fn detects_underline_adorned_headings() {
        let content = "Title\n=====\n\nBody text.\n\nSubtitle\n--------\n\nMore text.\n";
        let headings = detect_plain_text_headings(content);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].title, "Title");
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].line_span, 2);
        assert_eq!(headings[1].title, "Subtitle");
        assert_eq!(headings[1].level, 2);
    }

    #[test]
    fn detects_all_caps_headings() {
        let content = "GETTING STARTED\n\nSome text here.\n\nINSTALLATION GUIDE\n\nMore text.\n";
        let headings = detect_plain_text_headings(content);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].title, "Getting Started");
        assert_eq!(headings[1].title, "Installation Guide");
    }

    #[test]
    fn detects_numbered_headings() {
        let content = "1. Introduction\n\nText.\n\n2. Setup\n\nMore text.\n";
        let headings = detect_plain_text_headings(content);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].title, "Introduction");
        assert_eq!(headings[1].title, "Setup");
    }

    #[test]
    fn mixed_heading_styles() {
        let content = "\
PROJECT OVERVIEW\n\
\n\
Some intro.\n\
\n\
Details\n\
-------\n\
\n\
The details.\n\
\n\
1. First Step\n\
\n\
Do this.\n";
        let headings = detect_plain_text_headings(content);
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].title, "Project Overview");
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[1].title, "Details");
        assert_eq!(headings[2].title, "First Step");
        assert_eq!(headings[2].level, 2);
    }
}
