use super::*;

// ════════════════════════════════════════════════════════════════
// 39. MethodBase tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_method_base_exists() {
    let items = fixture_items();
    let mb = find_by_name(&items, "MethodBase");
    assert_eq!(mb.kind, SymbolKind::Class);
}

#[test]
fn class_method_base_has_methods() {
    let items = fixture_items();
    let mb = find_by_name(&items, "MethodBase");
    assert!(
        mb.metadata.methods.contains(&"normal_virtual".to_string()),
        "MethodBase should have normal_virtual method"
    );
}
