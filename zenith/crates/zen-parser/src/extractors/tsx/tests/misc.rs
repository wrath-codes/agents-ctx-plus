use super::*;

// ── Line numbers ───────────────────────────────────────────────

#[test]
fn line_numbers_are_one_based() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    for item in &items {
        assert!(
            item.start_line >= 1,
            "'{}' start_line should be >= 1, got {}",
            item.name,
            item.start_line
        );
        assert!(
            item.end_line >= item.start_line,
            "'{}' end_line {} < start_line {}",
            item.name,
            item.end_line,
            item.start_line
        );
    }
}

// ── All items extracted (smoke test) ───────────────────────────

#[test]
fn total_item_count() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    // Expanded fixture has many more items now.
    assert!(
        items.len() >= 28,
        "expected >= 28 items, got {}: {:?}",
        items.len(),
        items.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}
