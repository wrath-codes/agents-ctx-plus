use super::*;

#[test]
fn constants_module_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Constants");
    assert_eq!(m.kind, SymbolKind::Module);
    assert!(
        m.doc_comment.contains("constants"),
        "doc: {:?}",
        m.doc_comment
    );
}

#[test]
fn constants_module_methods() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "Sample.Constants");
    assert!(
        m.metadata.methods.contains(&"max_retries".to_string()),
        "methods: {:?}",
        m.metadata.methods
    );
    assert!(
        m.metadata.methods.contains(&"default_timeout".to_string()),
        "methods: {:?}",
        m.metadata.methods
    );
}
