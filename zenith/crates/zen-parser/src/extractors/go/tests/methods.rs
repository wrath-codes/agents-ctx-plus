use super::*;

#[test]
fn pointer_receiver_method_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Run");
    assert_eq!(m.kind, SymbolKind::Method);
    assert_eq!(m.visibility, Visibility::Public);
    assert_eq!(m.metadata.for_type.as_deref(), Some("Config"));
    assert!(m
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "receiver:pointer"));
}

#[test]
fn pointer_receiver_method_has_doc() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Run");
    assert!(
        m.doc_comment.contains("executes"),
        "doc: {:?}",
        m.doc_comment
    );
}

#[test]
fn value_receiver_method_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "String");
    assert_eq!(m.kind, SymbolKind::Method);
    assert_eq!(m.metadata.for_type.as_deref(), Some("Config"));
}
