use super::*;

#[test]
fn type_alias_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "MyInt");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
    assert_eq!(t.visibility, Visibility::Public);
}

#[test]
fn function_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "Callback");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
    assert_eq!(t.visibility, Visibility::Public);
}

#[test]
fn bare_type_declaration_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let d = find_by_name(&items, "Direction");
    assert_eq!(d.kind, SymbolKind::TypeAlias);
}

#[test]
fn map_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "StringMap");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
}

#[test]
fn channel_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "EventChan");
    assert_eq!(t.kind, SymbolKind::TypeAlias);
}
