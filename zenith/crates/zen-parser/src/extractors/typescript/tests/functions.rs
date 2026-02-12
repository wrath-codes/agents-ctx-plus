use super::*;

#[test]
fn exported_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Export);
    assert!(f.metadata.is_async);
    assert!(f.metadata.is_exported);
}

#[test]
fn non_exported_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "internalHelper");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Private);
    assert!(!f.metadata.is_exported);
}

#[test]
fn async_function_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert!(f.metadata.is_async);
    let helper = find_by_name(&items, "internalHelper");
    assert!(!helper.metadata.is_async);
}

#[test]
fn return_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert!(
        f.metadata.return_type.is_some(),
        "processItems should have return type"
    );
    let rt = f.metadata.return_type.as_deref().unwrap();
    assert!(
        rt.contains("Promise"),
        "return type should contain Promise: {rt}"
    );
}

#[test]
fn type_parameters_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert!(
        f.metadata.type_parameters.is_some(),
        "processItems should have type params"
    );
    let tp = f.metadata.type_parameters.as_deref().unwrap();
    assert!(tp.contains('T'), "type params: {tp}");
}
