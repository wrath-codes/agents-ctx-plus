use super::*;

// ════════════════════════════════════════════════════════════════
// 40. TemplateMethodHost tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_template_method_host_exists() {
    let items = fixture_items();
    let tmh = find_by_name(&items, "TemplateMethodHost");
    assert_eq!(tmh.kind, SymbolKind::Class);
}
