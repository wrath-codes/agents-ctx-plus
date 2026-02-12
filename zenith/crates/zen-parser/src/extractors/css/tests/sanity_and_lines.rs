use super::*;

#[test]
fn line_numbers_monotonic() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    for item in &items {
        assert!(
            item.end_line >= item.start_line,
            "end_line should be >= start_line for '{}': {} < {}",
            item.name,
            item.end_line,
            item.start_line
        );
    }
}

#[test]
fn line_numbers_positive() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    for item in &items {
        assert!(
            item.start_line > 0,
            "start_line should be > 0 for '{}'",
            item.name
        );
    }
}

// ── Signature tests ────────────────────────────────────────────

#[test]
fn empty_stylesheet() {
    let items = parse_and_extract("");
    assert!(items.is_empty());
}

#[test]
fn comment_only_stylesheet() {
    let items = parse_and_extract("/* just a comment */");
    assert!(items.is_empty());
}
