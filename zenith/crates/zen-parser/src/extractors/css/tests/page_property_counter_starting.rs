use super::*;

#[test]
fn page_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "@page");
    assert_eq!(p.kind, SymbolKind::Module);
    assert_eq!(p.metadata.at_rule_name.as_deref(), Some("page"));
}

#[test]
fn page_has_properties() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "@page");
    assert!(
        p.metadata
            .css_properties
            .iter()
            .any(|prop| prop.contains("margin")),
        "page should have margin property: {:?}",
        p.metadata.css_properties
    );
}

// ── @property tests ────────────────────────────────────────────

#[test]
fn property_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "@property --gradient-angle");
    assert_eq!(p.kind, SymbolKind::Module);
    assert_eq!(p.metadata.at_rule_name.as_deref(), Some("property"));
}

#[test]
fn property_has_declarations() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "@property --gradient-angle");
    assert!(
        p.metadata
            .css_properties
            .iter()
            .any(|prop| prop.contains("syntax")),
        "property should have syntax declaration: {:?}",
        p.metadata.css_properties
    );
}

// ── @counter-style tests ───────────────────────────────────────

#[test]
fn counter_style_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let cs = find_by_name(&items, "@counter-style thumbs");
    assert_eq!(cs.kind, SymbolKind::Module);
    assert_eq!(cs.metadata.at_rule_name.as_deref(), Some("counter-style"));
}

#[test]
fn counter_style_has_properties() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let cs = find_by_name(&items, "@counter-style thumbs");
    assert!(
        cs.metadata
            .css_properties
            .iter()
            .any(|prop| prop.contains("system")),
        "counter-style should have system property: {:?}",
        cs.metadata.css_properties
    );
}

// ── @scope tests ───────────────────────────────────────────────

#[test]
fn starting_style_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let ss = find_by_name(&items, "@starting-style");
    assert_eq!(ss.kind, SymbolKind::Module);
    assert_eq!(ss.metadata.at_rule_name.as_deref(), Some("starting-style"));
}

#[test]
fn starting_style_nested_rules() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    // The .fade-in rule inside @starting-style should be extracted
    assert!(
        items.iter().any(|i| i.name.contains("fade-in")),
        "should find .fade-in nested inside @starting-style"
    );
}

// ── Inline tests for new constructs ────────────────────────────

#[test]
fn inline_page() {
    let items = parse_and_extract("@page { margin: 1in; }");
    let p = find_by_name(&items, "@page");
    assert_eq!(p.kind, SymbolKind::Module);
}

#[test]
fn inline_property() {
    let items = parse_and_extract(
        "@property --my-bg { syntax: '<color>'; inherits: false; initial-value: white; }",
    );
    let p = find_by_name(&items, "@property --my-bg");
    assert_eq!(p.kind, SymbolKind::Module);
}

#[test]
fn inline_counter_style() {
    let items = parse_and_extract("@counter-style stars { system: cyclic; symbols: \"★\"; }");
    let cs = find_by_name(&items, "@counter-style stars");
    assert_eq!(cs.kind, SymbolKind::Module);
}

#[test]
fn inline_starting_style() {
    let items = parse_and_extract("@starting-style { .box { scale: 0; } }");
    let ss = find_by_name(&items, "@starting-style");
    assert_eq!(ss.kind, SymbolKind::Module);
}
