use super::*;

#[test]
fn generator_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "generateNumbers");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(f.metadata.is_generator);
    assert!(!f.metadata.is_async);
}

#[test]
fn generator_function_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "generateNumbers");
    assert!(
        f.doc_comment.contains("Generate sequential numbers"),
        "doc: {:?}",
        f.doc_comment
    );
}

#[test]
fn generator_jsdoc_yields_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "generateNumbers");
    assert!(
        f.metadata.doc_sections.yields.is_some(),
        "should have @yields"
    );
}

#[test]
fn async_generator_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "asyncStream");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(f.metadata.is_generator);
    assert!(f.metadata.is_async);
}

#[test]
fn generator_function_signature_has_star() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "generateNumbers");
    assert!(
        f.signature.contains("function*") || f.signature.contains("function *"),
        "generator sig should contain '*': {}",
        f.signature
    );
}

#[test]
fn generator_function_parameters() {
    let source = include_str!("../../../../tests/fixtures/sample.js");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "generateNumbers");
    assert!(
        f.metadata.parameters.contains(&"max".to_string()),
        "params: {:?}",
        f.metadata.parameters
    );
}
