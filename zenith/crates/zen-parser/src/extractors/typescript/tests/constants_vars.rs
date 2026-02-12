use super::*;

#[test]
fn const_declaration_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "MAX_RETRIES");
    assert_eq!(c.kind, SymbolKind::Const);
    assert_eq!(c.visibility, Visibility::Export);
}

#[test]
fn non_exported_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "INTERNAL_TIMEOUT");
    assert_eq!(c.kind, SymbolKind::Const);
    assert_eq!(c.visibility, Visibility::Private);
}

#[test]
fn let_variable_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "counter");
    assert_eq!(c.kind, SymbolKind::Const);
    assert_eq!(c.visibility, Visibility::Private);
}

#[test]
fn var_variable_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "legacyFlag");
    assert_eq!(v.kind, SymbolKind::Const);
    assert_eq!(v.visibility, Visibility::Private);
}
