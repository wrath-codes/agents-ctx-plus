use super::*;

#[test]
fn form_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "contact-form");
    assert_eq!(f.kind, SymbolKind::Struct);
    assert_eq!(f.metadata.tag_name.as_deref(), Some("form"));
}

#[test]
fn form_action_in_attributes() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "contact-form");
    assert!(
        f.metadata
            .html_attributes
            .iter()
            .any(|(n, v)| n == "action" && v.as_deref() == Some("/api/contact")),
        "attrs: {:?}",
        f.metadata.html_attributes
    );
}

#[test]
fn form_method_in_attributes() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "contact-form");
    assert!(
        f.metadata
            .html_attributes
            .iter()
            .any(|(n, v)| n == "method" && v.as_deref() == Some("POST")),
        "attrs: {:?}",
        f.metadata.html_attributes
    );
}

#[test]
fn form_input_with_id_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let input = find_by_name(&items, "name-input");
    assert_eq!(input.metadata.tag_name.as_deref(), Some("input"));
}

// ── Template tests ─────────────────────────────────────────────

#[test]
fn template_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "card-template");
    assert_eq!(t.kind, SymbolKind::Struct);
    assert_eq!(t.metadata.tag_name.as_deref(), Some("template"));
}

// ── Dialog tests ───────────────────────────────────────────────

#[test]
fn dialog_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let d = find_by_name(&items, "confirm-dialog");
    assert_eq!(d.kind, SymbolKind::Struct);
    assert_eq!(d.metadata.tag_name.as_deref(), Some("dialog"));
}

#[test]
fn dialog_classes() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let d = find_by_name(&items, "confirm-dialog");
    assert!(
        d.metadata.class_names.contains(&"modal".to_string()),
        "classes: {:?}",
        d.metadata.class_names
    );
}

// ── Details tests ──────────────────────────────────────────────

#[test]
fn details_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let d = find_by_name(&items, "faq-section");
    assert_eq!(d.kind, SymbolKind::Struct);
    assert_eq!(d.metadata.tag_name.as_deref(), Some("details"));
}

// ── Script tests ───────────────────────────────────────────────

#[test]
fn table_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "data-table");
    assert_eq!(t.kind, SymbolKind::Struct);
    assert_eq!(t.metadata.tag_name.as_deref(), Some("table"));
}

#[test]
fn table_classes() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "data-table");
    assert!(
        t.metadata.class_names.contains(&"striped".to_string()),
        "classes: {:?}",
        t.metadata.class_names
    );
}

// ── Iframe tests ───────────────────────────────────────────────
