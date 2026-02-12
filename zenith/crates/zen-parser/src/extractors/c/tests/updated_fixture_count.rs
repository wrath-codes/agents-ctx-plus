use super::*;

// ── Updated fixture count ─────────────────────────────────────

#[test]
fn fixture_total_item_count() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    assert!(
        items.len() >= 90,
        "expected 90+ items with new constructs, got {}",
        items.len()
    );
}
