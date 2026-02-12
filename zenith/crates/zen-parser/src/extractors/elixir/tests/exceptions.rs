use super::*;

#[test]
fn defexception_module_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.AppError");
    assert_eq!(m.kind, SymbolKind::Module);
    assert!(m.metadata.is_error_type, "should be marked as error type");
}

#[test]
fn defexception_module_has_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.AppError");
    assert!(
        m.metadata.fields.contains(&"message".to_string()),
        "fields: {:?}",
        m.metadata.fields
    );
    assert!(
        m.metadata.fields.contains(&"code".to_string()),
        "fields: {:?}",
        m.metadata.fields
    );
}

#[test]
fn defexception_standalone_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let structs: Vec<_> = items
        .iter()
        .filter(|i| i.kind == SymbolKind::Struct && i.name == "defexception")
        .collect();
    assert!(!structs.is_empty(), "should extract defexception as struct");
    assert!(structs[0].metadata.is_error_type, "should be error type");
    assert!(
        structs[0].metadata.fields.contains(&"message".to_string()),
        "fields: {:?}",
        structs[0].metadata.fields
    );
}

#[test]
fn defexception_module_doc() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.AppError");
    assert_eq!(m.doc_comment, "Application error.");
}

#[test]
fn defexception_module_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.AppError");
    assert!(
        m.metadata.methods.contains(&"from_code".to_string()),
        "methods: {:?}",
        m.metadata.methods
    );
}
