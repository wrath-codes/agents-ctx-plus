use super::*;

#[test]
fn custom_properties_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let cp = find_by_name(&items, "--primary-color");
    assert_eq!(cp.kind, SymbolKind::Const);
    assert!(cp.metadata.is_custom_property);
    assert!(cp.signature.contains("#3498db"));
}

#[test]
fn custom_property_spacing() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let cp = find_by_name(&items, "--spacing-md");
    assert_eq!(cp.kind, SymbolKind::Const);
    assert!(cp.signature.contains("1rem"));
}

#[test]
fn custom_property_count() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    assert_eq!(
        items
            .iter()
            .filter(|i| i.metadata.is_custom_property)
            .count(),
        7,
        "should find 7 CSS custom properties"
    );
}

// ── Element selector tests ─────────────────────────────────────

#[test]
fn simple_custom_property() {
    let items = parse_and_extract(":root { --gap: 10px; }");
    let cp = find_by_name(&items, "--gap");
    assert_eq!(cp.kind, SymbolKind::Const);
    assert!(cp.metadata.is_custom_property);
}

// ── Universal selector tests ──────────────────────────────────

#[test]
fn universal_selector_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let u = find_by_name(&items, "*");
    assert_eq!(u.kind, SymbolKind::Class);
    assert_eq!(u.metadata.selector.as_deref(), Some("*"));
}

#[test]
fn universal_selector_inline() {
    let items = parse_and_extract("* { margin: 0; }");
    let u = find_by_name(&items, "*");
    assert_eq!(u.kind, SymbolKind::Class);
}

// ── Modern pseudo-function tests ───────────────────────────────
