use super::*;

#[test]
fn element_selector_body() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let b = find_by_name(&items, "body");
    assert_eq!(b.kind, SymbolKind::Class);
    assert_eq!(b.metadata.selector.as_deref(), Some("body"));
}

#[test]
fn element_selector_properties() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let b = find_by_name(&items, "body");
    assert!(
        b.metadata
            .css_properties
            .iter()
            .any(|p| p.starts_with("margin")),
        "body should have margin property: {:?}",
        b.metadata.css_properties
    );
}

// ── Class selector tests ───────────────────────────────────────

#[test]
fn class_selector_card() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, ".card");
    assert_eq!(c.kind, SymbolKind::Class);
    assert_eq!(c.metadata.selector.as_deref(), Some(".card"));
}

#[test]
fn class_selector_bem_pattern() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let ct = find_by_name(&items, ".card__title");
    assert_eq!(ct.kind, SymbolKind::Class);
}

#[test]
fn class_selector_modifier() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let bp = find_by_name(&items, ".btn--primary");
    assert_eq!(bp.kind, SymbolKind::Class);
}

// ── ID selector tests ──────────────────────────────────────────

#[test]
fn id_selector_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let h = find_by_name(&items, "#main-header");
    assert_eq!(h.kind, SymbolKind::Static);
    assert_eq!(h.metadata.selector.as_deref(), Some("#main-header"));
}

#[test]
fn id_selector_sidebar() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "#sidebar");
    assert_eq!(s.kind, SymbolKind::Static);
}

// ── Pseudo-class tests ─────────────────────────────────────────

#[test]
fn pseudo_class_hover() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let h = find_by_name(&items, "a:hover");
    assert_eq!(h.kind, SymbolKind::Class);
}

#[test]
fn pseudo_class_focus() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, ".btn:focus");
    assert_eq!(f.kind, SymbolKind::Class);
}

// ── Pseudo-element tests ───────────────────────────────────────

#[test]
fn pseudo_element_first_line() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "p::first-line");
    assert_eq!(p.kind, SymbolKind::Class);
}

#[test]
fn pseudo_element_after() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, ".tooltip::after");
    assert_eq!(t.kind, SymbolKind::Class);
}

// ── Combinator tests ───────────────────────────────────────────

#[test]
fn descendant_selector() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let d = find_by_name(&items, ".parent .child");
    assert_eq!(d.kind, SymbolKind::Class);
}

#[test]
fn child_selector() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, ".parent > .direct-child");
    assert_eq!(c.kind, SymbolKind::Class);
}

#[test]
fn adjacent_sibling_selector() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let a = find_by_name(&items, ".sibling + .adjacent");
    assert_eq!(a.kind, SymbolKind::Class);
}

#[test]
fn general_sibling_selector() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let g = find_by_name(&items, ".sibling ~ .general");
    assert_eq!(g.kind, SymbolKind::Class);
}

// ── Multiple selectors test ────────────────────────────────────

#[test]
fn multiple_selectors() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "h1, h2, h3, h4");
    assert_eq!(m.kind, SymbolKind::Class);
}

// ── @media tests ───────────────────────────────────────────────

#[test]
fn complex_hover_selector() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, ".card:hover .card__title");
    assert_eq!(c.kind, SymbolKind::Class);
}

#[test]
fn attribute_selector() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let a = find_by_name(&items, "[data-tooltip]");
    assert_eq!(a.kind, SymbolKind::Class);
}

// ── Line number tests ──────────────────────────────────────────

#[test]
fn rule_signature_includes_selector() {
    let source = ".card { display: flex; }";
    let items = parse_and_extract(source);
    let c = find_by_name(&items, ".card");
    assert!(
        c.signature.starts_with(".card"),
        "signature should start with selector: {}",
        c.signature
    );
}

#[test]
fn rule_signature_includes_properties() {
    let source = ".card { display: flex; color: red; }";
    let items = parse_and_extract(source);
    let c = find_by_name(&items, ".card");
    assert!(
        c.signature.contains("display: flex"),
        "signature should include properties: {}",
        c.signature
    );
}

// ── Source extraction tests ─────────────────────────────────────

#[test]
fn source_present_for_rule() {
    let source = ".test { color: red; }";
    let items = parse_and_extract(source);
    let t = find_by_name(&items, ".test");
    assert!(t.source.is_some(), "source should be present");
}

// ── Inline tests (no fixture) ──────────────────────────────────

#[test]
fn simple_class_rule() {
    let items = parse_and_extract(".btn { padding: 8px; }");
    let b = find_by_name(&items, ".btn");
    assert_eq!(b.kind, SymbolKind::Class);
    assert!(
        b.metadata
            .css_properties
            .iter()
            .any(|p| p.contains("padding"))
    );
}

#[test]
fn simple_id_rule() {
    let items = parse_and_extract("#app { width: 100%; }");
    let a = find_by_name(&items, "#app");
    assert_eq!(a.kind, SymbolKind::Static);
}
