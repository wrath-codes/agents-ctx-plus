use super::*;

// ════════════════════════════════════════════════════════════════
// 34. Decltype return type tests
// ════════════════════════════════════════════════════════════════

#[test]
fn decltype_example_exists() {
    let items = fixture_items();
    let de = find_by_name(&items, "decltype_example");
    assert_eq!(de.kind, SymbolKind::Function);
}

#[test]
fn decltype_in_return_type() {
    let items = parse_and_extract("auto foo(int a) -> decltype(a) { return a; }");
    let f = find_by_name(&items, "foo");
    assert_eq!(f.kind, SymbolKind::Function);
}
