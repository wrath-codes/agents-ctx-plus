use super::*;

#[test]
fn covers_scalar_types_and_signatures() {
    let items = conformance_items();

    let answer = find_by_name(&items, "answer");
    assert_eq!(answer.metadata.return_type.as_deref(), Some("integer"));

    let pi = find_by_name(&items, "pi");
    assert_eq!(pi.metadata.return_type.as_deref(), Some("float"));

    let active = find_by_name(&items, "active");
    assert_eq!(active.metadata.return_type.as_deref(), Some("boolean"));

    let dob = find_by_name(&items, "dob");
    assert_eq!(dob.metadata.return_type.as_deref(), Some("local_date"));

    let meeting = find_by_name(&items, "meeting_time");
    assert_eq!(meeting.metadata.return_type.as_deref(), Some("local_time"));

    let ts = find_by_name(&items, "published_at");
    assert_eq!(ts.metadata.return_type.as_deref(), Some("offset_date_time"));
}

#[test]
fn covers_arrays_inline_tables_and_nested_arrays() {
    let items = conformance_items();

    let numbers = find_by_name(&items, "numbers");
    assert!(numbers
        .metadata
        .attributes
        .iter()
        .any(|a| a == "toml:array_elements:integer"));

    let mixed = find_by_name(&items, "mixed_numbers");
    assert!(mixed
        .metadata
        .attributes
        .iter()
        .any(|a| a == "toml:array_mixed"));

    let matrix = find_by_name(&items, "matrix");
    assert!(matrix
        .metadata
        .attributes
        .iter()
        .any(|a| a == "toml:array_elements:array"));

    assert_eq!(find_by_name(&items, "point.x").kind, SymbolKind::Property);
    assert_eq!(
        find_by_name(&items, "shape.size.w").kind,
        SymbolKind::Property
    );
}

#[test]
fn preserves_quoted_keys_and_unicode_escapes() {
    let items = conformance_items();
    assert_eq!(find_by_name(&items, "a.b").kind, SymbolKind::Property);
    assert_eq!(find_by_name(&items, "unicodeα").kind, SymbolKind::Property);
}

#[test]
fn handles_comments_without_false_positive_inside_strings() {
    let items = conformance_items();
    let literal = find_by_name(&items, "literal");
    let multi = find_by_name(&items, "multiline");

    assert!(literal.doc_comment.is_empty());
    assert!(multi.doc_comment.is_empty());
}

#[test]
fn extracts_tables_dotted_tables_and_array_tables() {
    let items = conformance_items();

    assert_eq!(find_by_name(&items, "owner").kind, SymbolKind::Module);
    assert_eq!(find_by_name(&items, "owner.child").kind, SymbolKind::Module);
    assert_eq!(find_by_name(&items, "products[0]").kind, SymbolKind::Module);
    assert_eq!(find_by_name(&items, "products[1]").kind, SymbolKind::Module);
    assert_eq!(
        find_by_name(&items, "products[0].name").kind,
        SymbolKind::Property
    );
    assert_eq!(
        find_by_name(&items, "products[1].price").kind,
        SymbolKind::Property
    );
}
