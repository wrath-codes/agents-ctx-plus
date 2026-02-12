use super::*;

#[test]
fn async_function_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "fetchData");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(f.metadata.is_async);
}
