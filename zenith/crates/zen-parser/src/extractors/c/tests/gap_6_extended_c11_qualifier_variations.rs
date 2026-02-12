use super::*;

// ── Gap 6 extended: C11 qualifier variations ──────────────────

#[test]
fn restrict_variable_has_attr() {
    let items = parse_and_extract("restrict int *ptr;");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"restrict".to_string()),
        "should have restrict: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn noreturn_definition() {
    let items = parse_and_extract("_Noreturn void die(void) { while(1); }");
    assert_eq!(items[0].kind, SymbolKind::Function);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"_Noreturn".to_string())
    );
}

#[test]
fn atomic_with_init() {
    let items = parse_and_extract("_Atomic int shared = 0;");
    assert_eq!(items[0].kind, SymbolKind::Static);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"_Atomic".to_string())
    );
}
