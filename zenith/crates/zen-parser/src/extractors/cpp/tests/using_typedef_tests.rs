use super::*;

// ════════════════════════════════════════════════════════════════
// 9. Using / typedef tests
// ════════════════════════════════════════════════════════════════

#[test]
fn using_alias_string_vec() {
    let items = fixture_items();
    let sv = find_by_name(&items, "StringVec");
    assert_eq!(sv.kind, SymbolKind::TypeAlias);
    assert!(
        sv.metadata.attributes.contains(&"using".to_string()),
        "StringVec should have 'using' attribute"
    );
}

#[test]
fn using_alias_callback() {
    let items = fixture_items();
    let cb = find_by_name(&items, "Callback");
    assert_eq!(cb.kind, SymbolKind::TypeAlias);
}

#[test]
fn using_alias_size() {
    let items = fixture_items();
    let s = find_by_name(&items, "Size");
    assert_eq!(s.kind, SymbolKind::TypeAlias);
}

#[test]
fn using_alias_compare_func() {
    let items = fixture_items();
    let cf = find_by_name(&items, "CompareFunc");
    assert_eq!(cf.kind, SymbolKind::TypeAlias);
}

#[test]
fn typedef_old_callback() {
    let items = fixture_items();
    let oc = find_by_name(&items, "OldCallback");
    assert_eq!(oc.kind, SymbolKind::TypeAlias);
    assert!(
        oc.metadata.attributes.contains(&"typedef".to_string()),
        "OldCallback should have typedef attribute"
    );
}

#[test]
fn using_declaration_cout() {
    let items = fixture_items();
    let cout = items
        .iter()
        .find(|i| i.name.contains("cout") && i.kind == SymbolKind::Module);
    assert!(cout.is_some(), "using std::cout should be extracted");
}

#[test]
fn using_declaration_endl() {
    let items = fixture_items();
    let endl = items
        .iter()
        .find(|i| i.name.contains("endl") && i.kind == SymbolKind::Module);
    assert!(endl.is_some(), "using std::endl should be extracted");
}

#[test]
fn using_declaration_has_attribute() {
    let items = fixture_items();
    let cout = items
        .iter()
        .find(|i| i.name.contains("cout") && i.kind == SymbolKind::Module)
        .expect("cout should exist");
    assert!(
        cout.metadata
            .attributes
            .contains(&"using_declaration".to_string()),
        "using std::cout should have using_declaration attribute"
    );
}
