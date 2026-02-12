use super::*;

// ── Gap 4: register storage class ─────────────────────────────

#[test]
fn register_variable_has_attr() {
    let items = parse_and_extract("register int fast;");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"register".to_string()),
        "should have register attr: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn fixture_fast_counter_register() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let fc = find_by_name(&items, "fast_counter");
    assert!(
        fc.metadata.attributes.contains(&"register".to_string()),
        "fast_counter should have register: {:?}",
        fc.metadata.attributes
    );
}
