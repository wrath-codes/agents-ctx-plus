use super::*;

// ════════════════════════════════════════════════════════════════
// 20. VBase tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_vbase_exists() {
    let items = fixture_items();
    let vb = find_by_name(&items, "VBase");
    assert_eq!(vb.kind, SymbolKind::Class);
}

#[test]
fn class_vbase_has_destructor() {
    let items = fixture_items();
    let vb = find_by_name(&items, "VBase");
    assert!(
        vb.metadata.methods.iter().any(|m| m.contains('~')),
        "VBase should have destructor, got {:?}",
        vb.metadata.methods
    );
}

#[test]
fn class_vbase_has_base_val_field() {
    let items = fixture_items();
    let vb = find_by_name(&items, "VBase");
    assert!(
        vb.metadata.fields.contains(&"base_val".to_string()),
        "VBase should have base_val field, got {:?}",
        vb.metadata.fields
    );
}
