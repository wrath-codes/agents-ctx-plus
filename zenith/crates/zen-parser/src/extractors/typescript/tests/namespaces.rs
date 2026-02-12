use super::*;

#[test]
fn exported_namespace_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let ns = find_by_name(&items, "Validators");
    assert_eq!(ns.kind, SymbolKind::Module);
    assert_eq!(ns.visibility, Visibility::Export);
    assert!(ns.metadata.is_exported);
}

#[test]
fn non_exported_namespace_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let ns = find_by_name(&items, "InternalUtils");
    assert_eq!(ns.kind, SymbolKind::Module);
    assert_eq!(ns.visibility, Visibility::Private);
}

#[test]
fn namespace_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let ns = find_by_name(&items, "Validators");
    assert!(
        ns.doc_comment.contains("String validation"),
        "doc: {:?}",
        ns.doc_comment
    );
}
