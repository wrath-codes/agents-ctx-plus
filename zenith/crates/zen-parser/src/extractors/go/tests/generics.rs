use super::*;

#[test]
fn generic_struct_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "Pair");
    assert_eq!(p.kind, SymbolKind::Struct);
    assert!(
        p.metadata.type_parameters.is_some(),
        "should have type params"
    );
}

#[test]
fn generic_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "Map");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.type_parameters.is_some(),
        "should have type params"
    );
}
