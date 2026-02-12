use super::*;

// ════════════════════════════════════════════════════════════════
// 12. Function tests
// ════════════════════════════════════════════════════════════════

#[test]
fn function_fast_max_inline() {
    let items = fixture_items();
    let f = find_by_name(&items, "fast_max");
    assert_eq!(f.kind, SymbolKind::Function);
    assert!(
        f.metadata.attributes.contains(&"inline".to_string()),
        "fast_max should have inline attribute"
    );
}

#[test]
fn function_internal_helper_static() {
    let items = fixture_items();
    let f = find_by_name(&items, "internal_helper");
    assert_eq!(f.kind, SymbolKind::Function);
    assert_eq!(
        f.visibility,
        Visibility::Private,
        "static function should be private"
    );
}

#[test]
fn function_external_function_extern() {
    let items = fixture_items();
    let ef = find_by_name(&items, "external_function");
    assert_eq!(ef.kind, SymbolKind::Function);
    assert!(
        ef.metadata.attributes.contains(&"extern".to_string())
            || ef.metadata.attributes.contains(&"prototype".to_string()),
        "external_function should be extern or prototype"
    );
}

#[test]
fn function_process_data_params() {
    let items = fixture_items();
    let pd = find_by_name(&items, "process_data");
    assert_eq!(pd.kind, SymbolKind::Function);
    assert!(
        pd.metadata.parameters.len() >= 3,
        "process_data should have 3+ params, got {:?}",
        pd.metadata.parameters
    );
}

#[test]
fn function_trailing_return_auto() {
    let items = fixture_items();
    let tr = find_by_name(&items, "trailing_return");
    assert_eq!(tr.kind, SymbolKind::Function);
    assert!(
        tr.metadata.attributes.contains(&"auto".to_string()),
        "trailing_return should have auto attribute, got {:?}",
        tr.metadata.attributes
    );
}

#[test]
fn function_make_adder_exists() {
    let items = fixture_items();
    let ma = find_by_name(&items, "make_adder");
    assert_eq!(ma.kind, SymbolKind::Function);
}

#[test]
fn function_reveal_secret_exists() {
    let items = fixture_items();
    let rs = find_by_name(&items, "reveal_secret");
    assert_eq!(rs.kind, SymbolKind::Function);
}

#[test]
fn function_safe_divide_return_type() {
    let items = fixture_items();
    let sd = find_by_name(&items, "safe_divide");
    assert!(
        sd.metadata
            .return_type
            .as_deref()
            .unwrap_or("")
            .contains("int"),
        "safe_divide should return int, got {:?}",
        sd.metadata.return_type
    );
}

#[test]
fn function_increment_counter_in_anonymous_ns() {
    let items = fixture_items();
    assert!(
        items
            .iter()
            .any(|i| i.name == "increment_counter" && i.kind == SymbolKind::Function),
        "increment_counter in anonymous namespace should exist"
    );
}

#[test]
fn function_factorial_has_doc() {
    let items = fixture_items();
    let f = find_by_name(&items, "factorial");
    assert!(
        !f.doc_comment.is_empty(),
        "factorial should have a doc comment"
    );
}
