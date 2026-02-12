use super::*;

#[test]
fn function_with_jsdoc_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "sum");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Private);
    assert!(!f.metadata.is_async);
    assert!(!f.metadata.is_exported);
}

#[test]
fn function_jsdoc_content() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "sum");
    assert!(
        f.doc_comment.contains("Calculate the sum"),
        "doc: {:?}",
        f.doc_comment
    );
}

#[test]
fn function_jsdoc_params_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "sum");
    assert!(
        f.metadata.doc_sections.args.contains_key("numbers"),
        "args: {:?}",
        f.metadata.doc_sections.args
    );
}

#[test]
fn function_jsdoc_returns_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "sum");
    assert!(
        f.metadata.doc_sections.returns.is_some(),
        "should have @returns"
    );
}

#[test]
fn non_documented_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "internalHelper");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Private);
    assert!(f.doc_comment.is_empty());
}

#[test]
fn function_parameters_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "sum");
    assert!(
        f.metadata.parameters.contains(&"numbers".to_string()),
        "params: {:?}",
        f.metadata.parameters
    );
}
