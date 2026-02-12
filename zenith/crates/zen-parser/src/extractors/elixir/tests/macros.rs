use super::*;

#[test]
fn public_macro_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "define_handler");
    assert_eq!(m.kind, SymbolKind::Macro);
    assert_eq!(m.visibility, Visibility::Public);
}

#[test]
fn macro_doc_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "define_handler");
    assert!(
        m.doc_comment.contains("Define a handler"),
        "doc: {:?}",
        m.doc_comment
    );
}

#[test]
fn macro_params_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "define_handler");
    assert_eq!(m.metadata.parameters, vec!["name"]);
}

#[test]
fn private_macro_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "internal_macro");
    assert_eq!(m.kind, SymbolKind::Macro);
    assert_eq!(m.visibility, Visibility::Private);
}
