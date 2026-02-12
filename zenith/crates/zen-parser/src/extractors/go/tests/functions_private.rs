use super::*;

#[test]
fn private_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "privateHelper");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Private);
}

#[test]
fn init_function_private() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "init");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Private);
}
