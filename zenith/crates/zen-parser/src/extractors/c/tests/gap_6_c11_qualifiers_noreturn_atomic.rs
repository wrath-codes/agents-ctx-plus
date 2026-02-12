use super::*;

// ── Gap 6: C11 qualifiers (_Noreturn, _Atomic) ────────────────

#[test]
fn noreturn_function_has_attr() {
    let items = parse_and_extract("_Noreturn void die(void);");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"_Noreturn".to_string()),
        "should have _Noreturn: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn atomic_variable_has_attr() {
    let items = parse_and_extract("_Atomic int counter;");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"_Atomic".to_string()),
        "should have _Atomic: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn fixture_abort_with_message_noreturn() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let awm = find_by_name(&items, "abort_with_message");
    assert!(
        awm.metadata.attributes.contains(&"_Noreturn".to_string()),
        "abort_with_message should have _Noreturn: {:?}",
        awm.metadata.attributes
    );
}

#[test]
fn fixture_atomic_counter() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let ac = find_by_name(&items, "atomic_counter");
    assert!(
        ac.metadata.attributes.contains(&"_Atomic".to_string()),
        "atomic_counter should have _Atomic: {:?}",
        ac.metadata.attributes
    );
}
