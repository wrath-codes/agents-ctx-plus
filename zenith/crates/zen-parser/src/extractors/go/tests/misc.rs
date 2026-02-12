use super::*;

#[test]
fn exported_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "ProcessItems");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Public);
}

#[test]
fn exported_function_has_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "ProcessItems");
    assert!(
        f.doc_comment.contains("processes a slice"),
        "doc: {:?}",
        f.doc_comment
    );
}

#[test]
fn exported_function_parameters() {
    let source = include_str!("../../../../tests/fixtures/sample.go");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "ProcessItems");
    assert!(
        f.metadata.parameters.iter().any(|p| p.contains("items")),
        "params: {:?}",
        f.metadata.parameters
    );
}
