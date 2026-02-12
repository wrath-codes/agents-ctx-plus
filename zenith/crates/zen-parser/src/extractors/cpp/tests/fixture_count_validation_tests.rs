use super::*;

// ════════════════════════════════════════════════════════════════
// 42. Fixture count validation tests
// ════════════════════════════════════════════════════════════════

#[test]
fn fixture_has_unions() {
    let items = fixture_items();
    let unions = find_all_by_kind(&items, SymbolKind::Union);
    assert!(!unions.is_empty(), "expected at least 1 union (ShapeData)");
}

#[test]
fn fixture_total_items_increased() {
    let items = fixture_items();
    assert!(
        items.len() >= 60,
        "expected 60+ items with new features, got {}",
        items.len()
    );
}
