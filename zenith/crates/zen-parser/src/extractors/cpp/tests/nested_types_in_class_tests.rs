use super::*;

// ════════════════════════════════════════════════════════════════
// 28. Nested types in class tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_nesting_demo_exists() {
    let items = fixture_items();
    let nd = find_by_name(&items, "NestingDemo");
    assert_eq!(nd.kind, SymbolKind::Class);
}

#[test]
fn nested_enum_inner_status() {
    let items = fixture_items();
    let e = items
        .iter()
        .find(|i| i.kind == SymbolKind::Enum && i.name == "InnerStatus");
    assert!(e.is_some(), "nested enum InnerStatus should be extracted");
}

#[test]
fn nested_struct_inner_config() {
    let items = fixture_items();
    let s = items
        .iter()
        .find(|i| i.kind == SymbolKind::Struct && i.name == "InnerConfig");
    assert!(s.is_some(), "nested struct InnerConfig should be extracted");
}

#[test]
fn nested_class_inner() {
    let items = fixture_items();
    let inner = items
        .iter()
        .find(|i| i.kind == SymbolKind::Class && i.name == "Inner");
    assert!(inner.is_some(), "nested class Inner should be extracted");
}
