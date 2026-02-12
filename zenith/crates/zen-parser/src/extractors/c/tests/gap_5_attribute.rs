use super::*;

// ── Gap 5: __attribute__((…)) ─────────────────────────────────

#[test]
fn gcc_attribute_on_variable() {
    let items = parse_and_extract("__attribute__((unused)) static int x = 0;");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .iter()
            .any(|a| a.contains("__attribute__")),
        "should have __attribute__: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn gcc_attribute_on_function() {
    let items = parse_and_extract("__attribute__((noreturn)) void die(void);");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .iter()
            .any(|a| a.contains("__attribute__")),
        "should have __attribute__: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn fixture_attr_var() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let av = find_by_name(&items, "attr_var");
    assert!(
        av.metadata
            .attributes
            .iter()
            .any(|a| a.contains("__attribute__")),
        "attr_var should have __attribute__: {:?}",
        av.metadata.attributes
    );
}

#[test]
fn fixture_panic_handler_attr() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let ph = find_by_name(&items, "panic_handler");
    assert!(
        ph.metadata
            .attributes
            .iter()
            .any(|a| a.contains("noreturn")),
        "panic_handler should have noreturn attribute: {:?}",
        ph.metadata.attributes
    );
}
