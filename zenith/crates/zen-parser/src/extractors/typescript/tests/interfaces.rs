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

#[test]
fn interface_member_items_emitted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);

    let handle = find_by_name(&items, "Handler::handle");
    assert_eq!(handle.kind, SymbolKind::Method);

    let name = find_by_name(&items, "Handler::name");
    assert_eq!(name.kind, SymbolKind::Property);
}

#[test]
fn interface_indexer_and_event_members_emitted() {
    let source = r"
interface Bus {
  onMessage: (ev: MessageEvent) => void;
  [key: string]: unknown;
}
";
    let items = parse_and_extract(source);

    let event_member = find_by_name(&items, "Bus::onMessage");
    assert_eq!(event_member.kind, SymbolKind::Event);

    let indexer = find_by_name(&items, "Bus[]");
    assert_eq!(indexer.kind, SymbolKind::Indexer);
}
