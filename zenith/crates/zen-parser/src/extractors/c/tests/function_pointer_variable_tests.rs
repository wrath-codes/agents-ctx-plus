use super::*;

// ── Function pointer variable tests ───────────────────────────

#[test]
fn function_pointer_var_on_event() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let cb = find_by_name(&items, "on_event_callback");
    assert_eq!(cb.kind, SymbolKind::Static);
    assert!(
        cb.metadata
            .attributes
            .contains(&"function_pointer".to_string()),
        "should have function_pointer attr: {:?}",
        cb.metadata.attributes
    );
}

#[test]
fn function_pointer_var_cleanup() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let ch = find_by_name(&items, "cleanup_handler");
    assert_eq!(ch.kind, SymbolKind::Static);
    assert_eq!(ch.visibility, Visibility::Private);
}
