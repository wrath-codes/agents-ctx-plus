use super::*;

// ════════════════════════════════════════════════════════════════
// 32. Requires clause tests
// ════════════════════════════════════════════════════════════════

#[test]
fn requires_clause_checked_add() {
    let items = fixture_items();
    let f = find_by_name(&items, "checked_add");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"template".to_string()),
        "checked_add should be a template"
    );
}

#[test]
fn requires_clause_has_requires_attr() {
    let items = fixture_items();
    let f = find_by_name(&items, "checked_add");
    let has_requires = f
        .metadata
        .attributes
        .iter()
        .any(|a| a.starts_with("requires:"));
    assert!(
        has_requires,
        "checked_add should have requires: attribute, got {:?}",
        f.metadata.attributes
    );
}
