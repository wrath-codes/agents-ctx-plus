use super::*;

#[test]
fn genserver_module_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Worker");
    assert_eq!(m.kind, SymbolKind::Module);
}

#[test]
fn impl_callback_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let init_items: Vec<_> = items
        .iter()
        .filter(|i| i.name == "init" && i.kind == SymbolKind::Function)
        .collect();
    assert!(!init_items.is_empty(), "should find init callback");
    let init = init_items[0];
    assert_eq!(init.metadata.trait_name.as_deref(), Some("@impl"));
}

#[test]
fn handle_call_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    assert!(
        items.iter().any(|i| i.name == "handle_call"),
        "should find handle_call"
    );
}
