use super::*;

// ── Union tests ───────────────────────────────────────────────

#[test]
fn union_value_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let value = find_by_name(&items, "Value");
    assert_eq!(value.kind, SymbolKind::Union);
}

#[test]
fn union_value_has_fields() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let value = find_by_name(&items, "Value");
    assert!(
        value.metadata.fields.len() >= 4,
        "Value union should have 4+ fields: {:?}",
        value.metadata.fields
    );
}

#[test]
fn union_value_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let value = find_by_name(&items, "Value");
    assert!(
        value.doc_comment.contains("tagged value"),
        "expected doc about tagged value, got: {:?}",
        value.doc_comment
    );
}

#[test]
fn union_network_address() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let na = find_by_name(&items, "NetworkAddress");
    assert_eq!(na.kind, SymbolKind::Union);
    assert!(
        na.metadata.fields.len() >= 3,
        "NetworkAddress should have 3 fields: {:?}",
        na.metadata.fields
    );
}
