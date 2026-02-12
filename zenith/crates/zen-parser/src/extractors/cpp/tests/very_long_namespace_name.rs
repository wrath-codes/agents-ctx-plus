use super::*;

// ════════════════════════════════════════════════════════════════
// 38. very_long_namespace_name
// ════════════════════════════════════════════════════════════════

#[test]
fn very_long_namespace_exists() {
    let items = fixture_items();
    let ns = find_by_name(&items, "very_long_namespace_name");
    assert_eq!(ns.kind, SymbolKind::Module);
}
