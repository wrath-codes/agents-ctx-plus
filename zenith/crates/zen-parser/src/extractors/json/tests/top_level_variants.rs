use super::*;

#[test]
fn top_level_array_is_supported() {
    let items = parse_and_extract(r#"[{"id":1},{"id":2}]"#);

    let root = find_by_name(&items, "$");
    assert_eq!(root.metadata.return_type.as_deref(), Some("array"));

    let first = find_by_name(&items, "[0].id");
    assert_eq!(first.metadata.owner_name.as_deref(), Some("[0]"));

    let second = find_by_name(&items, "[1].id");
    assert_eq!(second.metadata.owner_name.as_deref(), Some("[1]"));
}

#[test]
fn primitive_array_elements_are_emitted() {
    let items = parse_and_extract(r#"[1,true,null,"x"]"#);

    let first = find_by_name(&items, "[0]");
    assert_eq!(first.metadata.return_type.as_deref(), Some("number"));
    assert!(first
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "json:array_element"));

    let second = find_by_name(&items, "[1]");
    assert_eq!(second.metadata.return_type.as_deref(), Some("boolean"));

    let third = find_by_name(&items, "[2]");
    assert_eq!(third.metadata.return_type.as_deref(), Some("null"));

    let fourth = find_by_name(&items, "[3]");
    assert_eq!(fourth.metadata.return_type.as_deref(), Some("string"));
}

#[test]
fn top_level_primitive_is_supported() {
    let items = parse_and_extract("42");
    let root = find_by_name(&items, "$");
    assert_eq!(root.kind, SymbolKind::Module);
    assert_eq!(root.metadata.return_type.as_deref(), Some("number"));
    assert_eq!(items.len(), 1);
}

#[test]
fn comments_are_ignored_when_present() {
    let source = "{\n// note\n\"a\": 1\n}";
    let items = parse_and_extract(source);
    let a = find_by_name(&items, "a");
    assert_eq!(a.metadata.return_type.as_deref(), Some("number"));
    assert!(a
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "json:nonstandard:comments"));
}
