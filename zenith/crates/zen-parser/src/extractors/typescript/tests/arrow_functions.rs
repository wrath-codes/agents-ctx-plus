use super::*;

#[test]
fn exported_arrow_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "fetchData");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Export);
    assert!(f.metadata.is_async);
}

#[test]
fn non_async_arrow_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "add");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Export);
    assert!(!f.metadata.is_async);
}

#[test]
fn non_exported_arrow_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "multiply");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Private);
}
