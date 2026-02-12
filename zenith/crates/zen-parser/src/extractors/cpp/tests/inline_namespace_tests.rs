use super::*;

// ════════════════════════════════════════════════════════════════
// 30. Inline namespace tests
// ════════════════════════════════════════════════════════════════

#[test]
fn inline_namespace_v2() {
    let items = fixture_items();
    let v2 = find_by_name(&items, "v2");
    assert_eq!(v2.kind, SymbolKind::Module);
    assert!(
        v2.metadata.attributes.contains(&"inline".to_string()),
        "v2 should have inline attribute, got {:?}",
        v2.metadata.attributes
    );
}

#[test]
fn inline_namespace_signature() {
    let items = fixture_items();
    let v2 = find_by_name(&items, "v2");
    assert!(
        v2.signature.contains("inline"),
        "inline namespace signature should contain 'inline', got {:?}",
        v2.signature
    );
}

#[test]
fn minimal_inline_namespace() {
    let items = parse_and_extract("inline namespace detail {}");
    let d = find_by_name(&items, "detail");
    assert_eq!(d.kind, SymbolKind::Module);
    assert!(d.metadata.attributes.contains(&"inline".to_string()));
}
