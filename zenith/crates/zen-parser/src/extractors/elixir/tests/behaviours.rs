use super::*;

#[test]
fn behaviour_module_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Behaviour");
    assert_eq!(m.kind, SymbolKind::Module);
}

#[test]
fn behaviour_callbacks_listed() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Behaviour");
    assert!(
        m.metadata
            .associated_types
            .contains(&"handle_event".to_string()),
        "callbacks: {:?}",
        m.metadata.associated_types
    );
    assert!(
        m.metadata
            .associated_types
            .contains(&"format_output".to_string()),
        "callbacks: {:?}",
        m.metadata.associated_types
    );
}
