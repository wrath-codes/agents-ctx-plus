use super::*;

// ════════════════════════════════════════════════════════════════
// 15. Struct tests
// ════════════════════════════════════════════════════════════════

#[test]
fn struct_counter_exists() {
    let items = fixture_items();
    let c = find_by_name(&items, "Counter");
    assert_eq!(c.kind, SymbolKind::Struct, "Counter should be Struct");
}

#[test]
fn struct_counter_has_value_field() {
    let items = fixture_items();
    let c = find_by_name(&items, "Counter");
    assert!(
        c.metadata.fields.contains(&"value".to_string()),
        "Counter should have value field, got {:?}",
        c.metadata.fields
    );
}

#[test]
fn struct_counter_has_doc_comment() {
    let items = fixture_items();
    let c = find_by_name(&items, "Counter");
    assert!(
        !c.doc_comment.is_empty(),
        "Counter should have a doc comment"
    );
}

#[test]
fn struct_point_in_namespace() {
    let items = fixture_items();
    let pts: Vec<_> = items
        .iter()
        .filter(|i| i.name == "Point" && i.kind == SymbolKind::Struct)
        .collect();
    assert!(!pts.is_empty(), "Point struct should exist");
    assert!(
        pts[0].metadata.fields.contains(&"x".to_string()),
        "Point should have x field"
    );
    assert!(
        pts[0].metadata.fields.contains(&"y".to_string()),
        "Point should have y field"
    );
}

#[test]
fn struct_pair_has_fields() {
    let items = fixture_items();
    let p = find_by_name(&items, "Pair");
    assert!(
        p.metadata.fields.contains(&"first".to_string()),
        "Pair should have first field"
    );
    assert!(
        p.metadata.fields.contains(&"second".to_string()),
        "Pair should have second field"
    );
}
