use super::*;

// ── Gap 7: anonymous struct/union in fields ───────────────────

#[test]
fn anonymous_union_field() {
    let items = parse_and_extract("struct TV { int tag; union { int i; float f; }; };");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .fields
            .contains(&"(anonymous union)".to_string()),
        "should have anonymous union field: {:?}",
        items[0].metadata.fields
    );
}

#[test]
fn anonymous_struct_field() {
    let items = parse_and_extract("struct Outer { struct { int x; int y; }; int z; };");
    assert!(
        items[0]
            .metadata
            .fields
            .contains(&"(anonymous struct)".to_string()),
        "should have anonymous struct field: {:?}",
        items[0].metadata.fields
    );
}

#[test]
fn anonymous_union_does_not_hide_named_fields() {
    let items = parse_and_extract("struct TV { int tag; union { int i; float f; }; };");
    assert!(
        items[0].metadata.fields.contains(&"tag".to_string()),
        "should still have 'tag' field: {:?}",
        items[0].metadata.fields
    );
}

#[test]
fn fixture_tagged_value_anonymous_union() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let tv = find_by_name(&items, "TaggedValue");
    assert_eq!(tv.kind, SymbolKind::Struct);
    assert!(
        tv.metadata
            .fields
            .contains(&"(anonymous union)".to_string()),
        "TaggedValue should have anonymous union: {:?}",
        tv.metadata.fields
    );
    assert!(
        tv.metadata.fields.contains(&"tag".to_string()),
        "TaggedValue should have 'tag': {:?}",
        tv.metadata.fields
    );
}
