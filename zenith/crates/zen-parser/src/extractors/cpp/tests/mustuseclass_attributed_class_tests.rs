use super::*;

// ════════════════════════════════════════════════════════════════
// 37. MustUseClass attributed class tests
// ════════════════════════════════════════════════════════════════

#[test]
fn must_use_class_exists() {
    let items = fixture_items();
    let mu = find_by_name(&items, "MustUseClass");
    assert_eq!(mu.kind, SymbolKind::Class);
}
