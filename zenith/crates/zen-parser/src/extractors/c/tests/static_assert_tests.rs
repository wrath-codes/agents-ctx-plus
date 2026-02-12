use super::*;

// ── Static assert tests ───────────────────────────────────────

#[test]
fn static_assert_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let asserts: Vec<_> = find_by_name_prefix(&items, "_Static_assert");
    assert!(
        asserts.len() >= 2,
        "should find at least 2 _Static_assert, got {}",
        asserts.len()
    );
}

#[test]
fn static_assert_has_attribute() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let sa = items
        .iter()
        .find(|i| i.name == "_Static_assert")
        .expect("should find _Static_assert");
    assert!(
        sa.metadata
            .attributes
            .contains(&"static_assert".to_string()),
        "should have static_assert attr"
    );
}
