use super::*;

#[test]
fn struct_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "Config");
    assert_eq!(s.kind, SymbolKind::Struct);
    assert_eq!(s.visibility, Visibility::Public);
}

#[test]
fn struct_fields_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "Config");
    assert!(
        s.metadata.fields.contains(&"Name".to_string()),
        "fields: {:?}",
        s.metadata.fields
    );
    assert!(
        s.metadata.fields.contains(&"Count".to_string()),
        "fields: {:?}",
        s.metadata.fields
    );
    assert!(
        s.metadata.fields.contains(&"Enabled".to_string()),
        "fields: {:?}",
        s.metadata.fields
    );
}

#[test]
fn struct_has_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "Config");
    assert!(
        s.doc_comment.contains("application configuration"),
        "doc: {:?}",
        s.doc_comment
    );
}

#[test]
fn embedded_type_in_struct_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "Server");
    assert_eq!(s.kind, SymbolKind::Struct);
    assert!(
        s.metadata.fields.contains(&"Config".to_string()),
        "should contain embedded Config: {:?}",
        s.metadata.fields
    );
}

#[test]
fn embedded_pointer_type_in_struct_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "Server");
    assert!(
        s.metadata.fields.contains(&"Logger".to_string()),
        "should contain embedded *Logger: {:?}",
        s.metadata.fields
    );
}

#[test]
fn named_fields_still_work_with_embedded() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "Server");
    assert!(
        s.metadata.fields.contains(&"Port".to_string()),
        "fields: {:?}",
        s.metadata.fields
    );
    assert!(
        s.metadata.fields.contains(&"Host".to_string()),
        "fields: {:?}",
        s.metadata.fields
    );
}
