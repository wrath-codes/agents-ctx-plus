use super::*;

#[test]
fn struct_module_has_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Config");
    assert!(
        m.metadata.fields.contains(&"name".to_string()),
        "fields: {:?}",
        m.metadata.fields
    );
    assert!(
        m.metadata.fields.contains(&"retries".to_string()),
        "fields: {:?}",
        m.metadata.fields
    );
    assert!(
        m.metadata.fields.contains(&"timeout".to_string()),
        "fields: {:?}",
        m.metadata.fields
    );
}

#[test]
fn defstruct_standalone_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let structs: Vec<_> = items
        .iter()
        .filter(|i| i.kind == SymbolKind::Struct)
        .collect();
    assert!(!structs.is_empty(), "should extract defstruct");
    assert!(
        structs[0].metadata.fields.contains(&"name".to_string()),
        "fields: {:?}",
        structs[0].metadata.fields
    );
}
