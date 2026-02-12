use super::*;

#[test]
fn arrow_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "multiply");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Private);
    assert!(!f.metadata.is_async);
}

#[test]
fn arrow_function_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "multiply");
    assert!(
        f.doc_comment.contains("Multiply two numbers"),
        "doc: {:?}",
        f.doc_comment
    );
}

#[test]
fn async_arrow_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "asyncTransform");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(f.metadata.is_async);
}
