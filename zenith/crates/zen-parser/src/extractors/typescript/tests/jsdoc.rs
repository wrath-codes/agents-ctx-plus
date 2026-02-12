use super::*;

#[test]
fn jsdoc_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert!(
        f.doc_comment.contains("Process a list of items"),
        "doc: {:?}",
        f.doc_comment
    );
}

#[test]
fn jsdoc_params_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert!(
        f.metadata.doc_sections.args.contains_key("items"),
        "args: {:?}",
        f.metadata.doc_sections.args
    );
    assert!(
        f.metadata.doc_sections.args.contains_key("handler"),
        "args: {:?}",
        f.metadata.doc_sections.args
    );
}

#[test]
fn jsdoc_returns_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert!(
        f.metadata.doc_sections.returns.is_some(),
        "should have @returns"
    );
}

#[test]
fn jsdoc_throws_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.ts");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "processItems");
    assert!(
        f.metadata.doc_sections.raises.contains_key("Error"),
        "raises: {:?}",
        f.metadata.doc_sections.raises
    );
}
