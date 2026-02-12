use super::*;

// ════════════════════════════════════════════════════════════════
// 36. Wrapper struct / deduction guide tests
// ════════════════════════════════════════════════════════════════

#[test]
fn template_wrapper_struct() {
    let items = fixture_items();
    let w = find_by_name(&items, "Wrapper");
    assert_eq!(w.kind, SymbolKind::Struct);
    assert!(
        w.metadata.attributes.contains(&"template".to_string()),
        "Wrapper should be a template struct"
    );
}
