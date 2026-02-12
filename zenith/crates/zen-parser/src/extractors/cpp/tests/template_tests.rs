use super::*;

// ════════════════════════════════════════════════════════════════
// 4. Template tests
// ════════════════════════════════════════════════════════════════

#[test]
fn template_generic_add_exists() {
    let items = fixture_items();
    let f = find_by_name(&items, "generic_add");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"template".to_string()),
        "generic_add should be a template"
    );
}

#[test]
fn template_generic_add_has_generics() {
    let items = fixture_items();
    let f = find_by_name(&items, "generic_add");
    assert!(
        f.metadata.generics.is_some(),
        "generic_add should have generics"
    );
}

#[test]
fn template_print_all_variadic() {
    let items = fixture_items();
    let f = find_by_name(&items, "print_all");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"template".to_string()),
        "print_all should be a template"
    );
    let generics = f.metadata.generics.as_deref().unwrap_or("");
    assert!(
        generics.contains("..."),
        "print_all should have variadic template params, got {generics}"
    );
}

#[test]
fn template_pair_struct() {
    let items = fixture_items();
    let p = find_by_name(&items, "Pair");
    assert_eq!(p.kind, SymbolKind::Struct);
    assert!(
        p.metadata.attributes.contains(&"template".to_string()),
        "Pair should be a template"
    );
}

#[test]
fn template_pair_has_generics() {
    let items = fixture_items();
    let p = find_by_name(&items, "Pair");
    assert!(p.metadata.generics.is_some(), "Pair should have generics");
}

#[test]
fn template_list_node_struct() {
    let items = fixture_items();
    let ln = find_by_name(&items, "ListNode");
    assert_eq!(ln.kind, SymbolKind::Struct);
    assert!(
        ln.metadata.attributes.contains(&"template".to_string()),
        "ListNode should be a template"
    );
}

#[test]
fn template_list_node_has_fields() {
    let items = fixture_items();
    let ln = find_by_name(&items, "ListNode");
    assert!(
        ln.metadata.fields.contains(&"data".to_string()),
        "ListNode should have data field, got {:?}",
        ln.metadata.fields
    );
}

#[test]
fn template_constrained_add() {
    let items = fixture_items();
    let f = find_by_name(&items, "constrained_add");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"template".to_string()),
        "constrained_add should be a template"
    );
}

#[test]
fn template_constrained_add_has_generics() {
    let items = fixture_items();
    let f = find_by_name(&items, "constrained_add");
    assert!(
        f.metadata.generics.is_some(),
        "constrained_add should have generics"
    );
}

#[test]
fn template_generic_add_parameters() {
    let items = fixture_items();
    let f = find_by_name(&items, "generic_add");
    assert!(
        f.metadata.parameters.len() >= 2,
        "generic_add should have 2+ parameters, got {:?}",
        f.metadata.parameters
    );
}
