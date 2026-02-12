use super::*;

// ════════════════════════════════════════════════════════════════
// 41. Structured binding tests
// ════════════════════════════════════════════════════════════════

#[test]
fn function_use_structured_bindings_exists() {
    let items = fixture_items();
    let usb = find_by_name(&items, "use_structured_bindings");
    assert_eq!(usb.kind, SymbolKind::Function);
}
