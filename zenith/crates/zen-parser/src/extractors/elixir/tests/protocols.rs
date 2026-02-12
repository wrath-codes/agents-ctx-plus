use super::*;

#[test]
fn defprotocol_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "Sample.Renderable");
    assert_eq!(p.kind, SymbolKind::Interface);
    assert_eq!(p.visibility, Visibility::Public);
}

#[test]
fn protocol_doc_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "Sample.Renderable");
    assert!(
        p.doc_comment.contains("rendering items"),
        "doc: {:?}",
        p.doc_comment
    );
}

#[test]
fn protocol_methods_listed() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "Sample.Renderable");
    assert!(
        p.metadata.methods.contains(&"render".to_string()),
        "methods: {:?}",
        p.metadata.methods
    );
}
