use super::*;

// ════════════════════════════════════════════════════════════════
// 14. Forward declaration tests
// ════════════════════════════════════════════════════════════════

#[test]
fn forward_decl_config_struct() {
    let items = fixture_items();
    let c = find_by_name(&items, "Config");
    assert_eq!(c.kind, SymbolKind::Struct, "Config should be Struct");
}

#[test]
fn forward_decl_widget_class() {
    let items = fixture_items();
    let w = find_by_name(&items, "Widget");
    assert_eq!(w.kind, SymbolKind::Class, "Widget should be Class");
}

#[test]
fn forward_decl_config_no_fields() {
    let items = fixture_items();
    let c = find_by_name(&items, "Config");
    assert!(
        c.metadata.fields.is_empty(),
        "forward decl Config should have no fields"
    );
}
