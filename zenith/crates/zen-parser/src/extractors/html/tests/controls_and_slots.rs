use super::*;

#[test]
fn fieldset_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "preferences");
    assert_eq!(f.kind, SymbolKind::Struct);
    assert_eq!(f.metadata.tag_name.as_deref(), Some("fieldset"));
}

#[test]
fn fieldset_classes() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "preferences");
    assert!(
        f.metadata.class_names.contains(&"pref-group".to_string()),
        "classes: {:?}",
        f.metadata.class_names
    );
}

#[test]
fn select_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "country-select");
    assert_eq!(s.kind, SymbolKind::Struct);
    assert_eq!(s.metadata.tag_name.as_deref(), Some("select"));
}

#[test]
fn output_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let o = find_by_name(&items, "calc-result");
    assert_eq!(o.kind, SymbolKind::Struct);
    assert_eq!(o.metadata.tag_name.as_deref(), Some("output"));
}

// ── Slot test ──────────────────────────────────────────────────

#[test]
fn slot_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let slots = find_all_by_tag(&items, "slot");
    assert!(!slots.is_empty(), "should find at least one slot element");
}

#[test]
fn named_slot_has_name_attr() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let slots = find_all_by_tag(&items, "slot");
    let named = slots.iter().find(|s| {
        s.metadata
            .html_attributes
            .iter()
            .any(|(n, v)| n == "name" && v.as_deref() == Some("sidebar-content"))
    });
    assert!(
        named.is_some(),
        "should find slot with name=sidebar-content"
    );
}
