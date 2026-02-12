use super::*;

// ── Gap 7 extended: anonymous aggregates ──────────────────────

#[test]
fn anonymous_struct_named_field_preserved() {
    let items = parse_and_extract("struct Outer { struct { int x; int y; }; int z; };");
    assert!(
        items[0].metadata.fields.contains(&"z".to_string()),
        "should preserve named field 'z': {:?}",
        items[0].metadata.fields
    );
}

#[test]
fn anonymous_union_in_typedef_struct() {
    let items =
        parse_and_extract("typedef struct { int tag; union { int i; double d; }; } Variant;");
    let v = find_by_name(&items, "Variant");
    assert_eq!(v.kind, SymbolKind::Struct);
    assert!(
        v.metadata.fields.contains(&"(anonymous union)".to_string()),
        "Variant should have anonymous union field: {:?}",
        v.metadata.fields
    );
}

#[test]
fn multiple_named_fields_with_anonymous() {
    let items = parse_and_extract("struct M { int a; union { int x; float y; }; int b; };");
    let m = find_by_name(&items, "M");
    assert!(m.metadata.fields.contains(&"a".to_string()));
    assert!(m.metadata.fields.contains(&"b".to_string()));
    assert!(m.metadata.fields.contains(&"(anonymous union)".to_string()));
}
