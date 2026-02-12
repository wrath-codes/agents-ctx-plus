use super::*;

// ════════════════════════════════════════════════════════════════
// 13. Lambda / variable tests
// ════════════════════════════════════════════════════════════════

#[test]
fn variable_doubler_exists() {
    let items = fixture_items();
    let d = find_by_name(&items, "doubler");
    assert!(
        d.kind == SymbolKind::Static || d.kind == SymbolKind::Const,
        "doubler should be Static or Const, got {:?}",
        d.kind
    );
}

#[test]
fn variable_doubler_auto_attr() {
    let items = fixture_items();
    let d = find_by_name(&items, "doubler");
    assert!(
        d.metadata.attributes.contains(&"auto".to_string()),
        "doubler should have auto attribute, got {:?}",
        d.metadata.attributes
    );
}

#[test]
fn variable_g_counter_exists() {
    let items = fixture_items();
    let g = find_by_name(&items, "g_counter");
    assert_eq!(g.kind, SymbolKind::Static, "g_counter should be Static");
}

#[test]
fn variable_s_instance_count_static() {
    let items = fixture_items();
    let s = find_by_name(&items, "s_instance_count");
    assert_eq!(
        s.kind,
        SymbolKind::Static,
        "s_instance_count should be Static"
    );
}

#[test]
fn variable_app_name_const() {
    let items = fixture_items();
    let a = find_by_name(&items, "APP_NAME");
    assert_eq!(a.kind, SymbolKind::Const, "APP_NAME should be Const");
}
