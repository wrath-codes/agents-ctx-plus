use super::*;

#[test]
fn error_struct_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let e = find_by_name(&items, "AppError");
    assert_eq!(e.kind, SymbolKind::Struct);
    assert!(e.metadata.is_error_type);
}

#[test]
fn error_method_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    // Error() method on *AppError
    let methods: Vec<_> = items
        .iter()
        .filter(|i| i.name == "Error" && i.kind == SymbolKind::Method)
        .collect();
    assert!(!methods.is_empty());
    let m = methods[0];
    assert_eq!(m.metadata.for_type.as_deref(), Some("*AppError"));
}
