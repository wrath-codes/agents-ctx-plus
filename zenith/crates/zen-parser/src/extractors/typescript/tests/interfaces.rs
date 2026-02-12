use super::*;

#[test]
fn exported_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Handler");
    assert_eq!(i.kind, SymbolKind::Interface);
    assert_eq!(i.visibility, Visibility::Export);
    assert!(i.metadata.is_exported);
}

#[test]
fn non_exported_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "PrivateConfig");
    assert_eq!(i.kind, SymbolKind::Interface);
    assert_eq!(i.visibility, Visibility::Private);
}

#[test]
fn interface_members_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Handler");
    assert!(i.metadata.methods.contains(&"handle".to_string()));
    assert!(i.metadata.methods.contains(&"name".to_string()));
}
