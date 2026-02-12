use super::*;

#[test]
fn interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Handler");
    assert_eq!(i.kind, SymbolKind::Interface);
    assert_eq!(i.visibility, Visibility::Public);
}

#[test]
fn interface_methods_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Handler");
    assert!(
        i.metadata.methods.contains(&"Handle".to_string()),
        "methods: {:?}",
        i.metadata.methods
    );
    assert!(
        i.metadata.methods.contains(&"Name".to_string()),
        "methods: {:?}",
        i.metadata.methods
    );
}

#[test]
fn interface_has_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Handler");
    assert!(
        i.doc_comment.contains("request handler"),
        "doc: {:?}",
        i.doc_comment
    );
}

#[test]
fn embedded_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Reader");
    assert_eq!(i.kind, SymbolKind::Interface);
}

#[test]
fn type_constraint_interface_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "Number");
    assert_eq!(i.kind, SymbolKind::Interface);
    assert_eq!(i.visibility, Visibility::Public);
}
