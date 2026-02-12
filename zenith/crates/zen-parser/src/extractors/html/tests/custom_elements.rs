use super::*;

#[test]
fn custom_element_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "my-component");
    assert_eq!(c.kind, SymbolKind::Component);
    assert!(c.metadata.is_custom_element);
}

#[test]
fn custom_element_attributes() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "my-component");
    assert!(
        c.metadata
            .html_attributes
            .iter()
            .any(|(n, _)| n == "data-id"),
        "attrs: {:?}",
        c.metadata.html_attributes
    );
}

#[test]
fn custom_element_x_modal() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "x-modal");
    assert_eq!(c.kind, SymbolKind::Component);
    assert!(c.metadata.is_custom_element);
}

#[test]
fn custom_element_app_header() {
    let source = include_str!("../../../../tests/fixtures/sample.html");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "app-header");
    assert_eq!(c.kind, SymbolKind::Component);
    assert!(c.metadata.is_custom_element);
}

// ── Elements with id tests ─────────────────────────────────────
