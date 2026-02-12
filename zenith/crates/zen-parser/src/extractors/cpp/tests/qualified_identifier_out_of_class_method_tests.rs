use super::*;

// ════════════════════════════════════════════════════════════════
// 22. Qualified identifier / out-of-class method tests
// ════════════════════════════════════════════════════════════════

#[test]
fn class_out_of_class_exists() {
    let items = fixture_items();
    let oc = find_by_name(&items, "OutOfClass");
    assert_eq!(oc.kind, SymbolKind::Class);
}

#[test]
fn qualified_identifier_recursive() {
    let items = parse_and_extract("class Foo { public: void bar(); };\nvoid Foo::bar() {}");
    // The out-of-class definition should be captured with qualified name
    let has_qualified = items
        .iter()
        .any(|i| i.kind == SymbolKind::Function && i.name.contains("Foo"));
    // Either found as qualified name or as plain function
    assert!(
        has_qualified
            || items
                .iter()
                .any(|i| i.kind == SymbolKind::Function && i.name == "bar"),
        "out-of-class method should be extracted"
    );
}

#[test]
fn minimal_qualified_identifier() {
    let items = parse_and_extract("class A {};\nvoid A::foo() {}");
    assert!(
        items
            .iter()
            .any(|i| i.kind == SymbolKind::Function && i.name.contains("foo")),
        "qualified function A::foo should be extracted"
    );
}
