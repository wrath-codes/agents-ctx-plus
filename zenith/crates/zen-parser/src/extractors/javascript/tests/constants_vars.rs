use super::*;

#[test]
fn const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "MAX_RETRIES");
    assert_eq!(c.kind, SymbolKind::Const);
    assert_eq!(c.visibility, Visibility::Private);
}

#[test]
fn let_variable_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "mutableCounter");
    assert_eq!(c.kind, SymbolKind::Static);
    assert_eq!(c.visibility, Visibility::Private);
}

#[test]
fn var_variable_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let v = find_by_name(&items, "legacyFlag");
    assert_eq!(v.kind, SymbolKind::Static);
    assert_eq!(v.visibility, Visibility::Private);
}
