use super::*;

// ════════════════════════════════════════════════════════════════
// 7. Concept tests
// ════════════════════════════════════════════════════════════════

#[test]
fn concept_stream_insertable() {
    let items = fixture_items();
    let si = find_by_name(&items, "StreamInsertable");
    assert_eq!(si.kind, SymbolKind::Trait);
    assert!(
        si.metadata.attributes.contains(&"concept".to_string()),
        "StreamInsertable should have concept attribute"
    );
}

#[test]
fn concept_stream_insertable_has_generics() {
    let items = fixture_items();
    let si = find_by_name(&items, "StreamInsertable");
    assert!(
        si.metadata.generics.is_some(),
        "StreamInsertable should have generics"
    );
}

#[test]
fn concept_addable() {
    let items = fixture_items();
    let a = find_by_name(&items, "Addable");
    assert_eq!(a.kind, SymbolKind::Trait);
    assert!(
        a.metadata.attributes.contains(&"concept".to_string()),
        "Addable should have concept attribute"
    );
}

#[test]
fn concept_addable_has_doc() {
    let items = fixture_items();
    let a = find_by_name(&items, "Addable");
    assert!(
        !a.doc_comment.is_empty(),
        "Addable should have a doc comment"
    );
}
