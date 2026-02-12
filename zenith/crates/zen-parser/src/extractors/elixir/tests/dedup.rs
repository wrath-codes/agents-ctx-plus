use super::*;

#[test]
fn multi_clause_deduped() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let classify_items = find_all_by_name(&items, "classify");
    assert_eq!(
        classify_items.len(),
        1,
        "multi-clause classify should be deduped to 1, found: {}",
        classify_items.len()
    );
}

#[test]
fn multi_clause_keeps_first_doc() {
    let source = include_str!("../../../../tests/fixtures/sample.ex");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "classify");
    assert!(
        f.doc_comment.contains("Classify a value"),
        "doc: {:?}",
        f.doc_comment
    );
}
