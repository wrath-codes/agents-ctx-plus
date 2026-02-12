use super::*;

#[test]
fn exported_type_alias_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "Result");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
    assert_eq!(t.visibility, Visibility::Export);
}

#[test]
fn non_exported_type_alias_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "InternalState");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
    assert_eq!(t.visibility, Visibility::Private);
}
