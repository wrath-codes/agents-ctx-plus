use super::*;

// ════════════════════════════════════════════════════════════════
// 23. C++11 Attribute tests
// ════════════════════════════════════════════════════════════════

#[test]
fn attributed_nodiscard_function() {
    let items = parse_and_extract("[[nodiscard]] int foo() { return 1; }");
    let f = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name == "foo");
    assert!(f.is_some(), "[[nodiscard]] function should be extracted");
}

#[test]
fn attributed_deprecated_function() {
    let items = parse_and_extract("[[deprecated]] void old() {}");
    let f = items
        .iter()
        .find(|i| i.kind == SymbolKind::Function && i.name == "old");
    assert!(f.is_some(), "[[deprecated]] function should be extracted");
}

#[test]
fn fixture_must_use_result_exists() {
    let items = fixture_items();
    let f = find_by_name(&items, "must_use_result");
    assert_eq!(f.kind, SymbolKind::Function);
}

#[test]
fn fixture_old_api_exists() {
    let items = fixture_items();
    let f = find_by_name(&items, "old_api");
    assert_eq!(f.kind, SymbolKind::Function);
}
