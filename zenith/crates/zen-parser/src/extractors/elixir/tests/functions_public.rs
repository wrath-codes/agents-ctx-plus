use super::*;

#[test]
fn public_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Public);
}

#[test]
fn function_doc_heredoc() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process");
    assert!(
        f.doc_comment.contains("Process a list of items."),
        "doc: {:?}",
        f.doc_comment
    );
}

#[test]
fn function_doc_inline() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process_one");
    assert_eq!(f.doc_comment, "Process a single item.");
}

#[test]
fn function_params_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process");
    assert_eq!(f.metadata.parameters, vec!["items"]);
}

#[test]
fn function_guard_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process");
    assert!(
        f.metadata.where_clause.is_some(),
        "should have guard clause"
    );
    assert!(
        f.metadata
            .where_clause
            .as_deref()
            .unwrap()
            .contains("is_list"),
        "guard: {:?}",
        f.metadata.where_clause
    );
}

#[test]
fn function_spec_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process");
    assert!(f.metadata.return_type.is_some(), "should have @spec");
    assert!(
        f.metadata.return_type.as_deref().unwrap().contains("list"),
        "spec: {:?}",
        f.metadata.return_type
    );
}

#[test]
fn function_doc_false_is_empty() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "internal_helper");
    assert_eq!(f.doc_comment, "");
}

#[test]
fn oneline_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "process_one");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(f.visibility, Visibility::Public);
}
