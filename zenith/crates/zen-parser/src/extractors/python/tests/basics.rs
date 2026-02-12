use super::*;

#[test]
fn extract_from_fixture() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"BaseProcessor"), "names: {names:?}");
    assert!(names.contains(&"Config"), "names: {names:?}");
    assert!(names.contains(&"fetch_data"), "names: {names:?}");
    assert!(names.contains(&"Validator"), "names: {names:?}");
}

#[test]
fn signature_no_body_leak() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    for item in &items {
        if !item.signature.is_empty() {
            assert!(
                !item.signature.contains("\n    pass")
                    && !item.signature.contains("\n    return")
                    && !item.signature.contains("\"\"\""),
                "signature for '{}' leaks body: {}",
                item.name,
                item.signature
            );
        }
    }
}

// ── New fixture coverage tests ─────────────────────────────────

#[test]
fn reasonable_item_count() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    // Should have a reasonable number of items (not inflated by duplicates)
    // Module + classes + functions + constants
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    assert!(
        items.len() >= 25,
        "should have at least 25 items, got {}: {:?}",
        items.len(),
        names
    );
    assert!(
        items.len() <= 140,
        "should not exceed 140 items (no runaway duplicates), got {}: {:?}",
        items.len(),
        names
    );
}
