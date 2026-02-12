use super::*;

// ── Array declaration tests ───────────────────────────────────

#[test]
fn array_lookup_table() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let lt = find_by_name(&items, "lookup_table");
    assert_eq!(lt.kind, SymbolKind::Static);
    assert!(
        lt.metadata.attributes.contains(&"array".to_string()),
        "should have array attr: {:?}",
        lt.metadata.attributes
    );
}

#[test]
fn array_prime_numbers_static() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let pn = find_by_name(&items, "prime_numbers");
    assert_eq!(pn.visibility, Visibility::Private);
    assert!(
        pn.metadata.attributes.contains(&"static".to_string()),
        "should have static attr"
    );
}
