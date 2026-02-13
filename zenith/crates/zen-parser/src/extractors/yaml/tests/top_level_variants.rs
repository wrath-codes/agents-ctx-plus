use super::*;

#[test]
fn top_level_scalar_is_supported() {
    let items = parse_and_extract("42\n");
    let root = find_by_name(&items, "$");
    assert_eq!(root.metadata.return_type.as_deref(), Some("number"));
    assert_eq!(items.len(), 1);
}

#[test]
fn comments_are_tagged_as_nonstandard() {
    let source = "# note\na: 1\n";
    let items = parse_and_extract(source);
    let key = find_by_name(&items, "a");
    assert!(
        key.metadata
            .attributes
            .iter()
            .any(|attr| attr == "yaml:nonstandard:comments")
    );
}
