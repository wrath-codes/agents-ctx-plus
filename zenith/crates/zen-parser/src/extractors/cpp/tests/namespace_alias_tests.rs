use super::*;

// ════════════════════════════════════════════════════════════════
// 26. Namespace alias tests
// ════════════════════════════════════════════════════════════════

#[test]
fn namespace_alias_vln() {
    let items = fixture_items();
    let vln = find_by_name(&items, "vln");
    assert_eq!(vln.kind, SymbolKind::Module);
    assert!(
        vln.metadata
            .attributes
            .contains(&"namespace_alias".to_string()),
        "vln should have namespace_alias attribute"
    );
}

#[test]
fn minimal_namespace_alias() {
    let items = parse_and_extract("namespace orig {} namespace alias = orig;");
    let a = find_by_name(&items, "alias");
    assert_eq!(a.kind, SymbolKind::Module);
    assert!(
        a.metadata
            .attributes
            .contains(&"namespace_alias".to_string())
    );
}
