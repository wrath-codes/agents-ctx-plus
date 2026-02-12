use super::*;

// ── Additional coverage: typedef union ────────────────────────

#[test]
fn typedef_union_extracted() {
    let items = parse_and_extract("typedef union { int i; float f; } NumericValue;");
    let nv = find_by_name(&items, "NumericValue");
    assert_eq!(nv.kind, SymbolKind::Union);
    assert!(nv.metadata.attributes.contains(&"typedef".to_string()));
    assert!(nv.metadata.fields.len() >= 2);
}
