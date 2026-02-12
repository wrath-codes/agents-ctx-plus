use super::*;

#[test]
fn constructor_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "NewConfig");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Public);
}
