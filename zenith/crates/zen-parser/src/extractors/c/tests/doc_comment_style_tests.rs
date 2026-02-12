use super::*;

// ── Doc comment style tests ───────────────────────────────────

#[test]
fn doc_comment_doxygen_style() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let add_def = items
        .iter()
        .find(|i| i.name == "add" && !i.metadata.attributes.contains(&"prototype".to_string()))
        .expect("should find add definition");
    assert!(
        add_def.doc_comment.contains("@param"),
        "should contain @param tags: {:?}",
        add_def.doc_comment
    );
}

#[test]
fn doc_comment_single_line_style() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let ll = find_by_name(&items, "LogLevel");
    assert!(
        ll.doc_comment.contains("Log level"),
        "LogLevel should have single-line doc: {:?}",
        ll.doc_comment
    );
}

#[test]
fn doc_comment_multiline_block() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let value = find_by_name(&items, "Value");
    assert!(
        !value.doc_comment.is_empty(),
        "Value should have a doc comment"
    );
}
