use super::*;

// ── Prototype tests ───────────────────────────────────────────

#[test]
fn prototype_add_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let proto = items
        .iter()
        .find(|i| i.name == "add" && i.metadata.attributes.contains(&"prototype".to_string()))
        .expect("should find add prototype");
    assert_eq!(proto.kind, SymbolKind::Function);
}

#[test]
fn prototype_process_data() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let proto = items
        .iter()
        .find(|i| {
            i.name == "process_data" && i.metadata.attributes.contains(&"prototype".to_string())
        })
        .expect("should find process_data prototype");
    assert_eq!(proto.kind, SymbolKind::Function);
}

#[test]
fn prototype_shutdown_subsystem() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let proto = items
        .iter()
        .find(|i| {
            i.name == "shutdown_subsystem"
                && i.metadata.attributes.contains(&"prototype".to_string())
        })
        .expect("should find shutdown_subsystem prototype");
    assert_eq!(proto.kind, SymbolKind::Function);
}
