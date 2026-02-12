use super::*;

#[test]
fn media_query_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let medias = find_all_by_at_rule(&items, "media");
    assert_eq!(medias.len(), 3, "should find 3 @media statements");
}

#[test]
fn media_query_name() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "@media (max-width: 768px)");
    assert_eq!(m.kind, SymbolKind::Module);
    assert_eq!(m.metadata.at_rule_name.as_deref(), Some("media"));
}

#[test]
fn media_query_nested_rules() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    // Nested rules inside @media should have parent context in name
    assert!(
        items.iter().any(|i| i.name.contains("@(max-width:")),
        "should find nested rules inside @media"
    );
}

#[test]
fn media_print_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "@media print");
    assert_eq!(m.kind, SymbolKind::Module);
}

// ── @keyframes tests ───────────────────────────────────────────

#[test]
fn container_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let containers = find_all_by_at_rule(&items, "container");
    assert_eq!(containers.len(), 1, "should find 1 @container query");
}

#[test]
fn container_kind() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let containers = find_all_by_at_rule(&items, "container");
    let c = containers.first().expect("should have @container");
    assert_eq!(c.kind, SymbolKind::Module);
}

// ── @supports tests ────────────────────────────────────────────

#[test]
fn supports_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let supports = find_all_by_at_rule(&items, "supports");
    assert_eq!(supports.len(), 2, "should find 2 @supports statements");
}

#[test]
fn supports_grid() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "@supports (display: grid)");
    assert_eq!(s.kind, SymbolKind::Module);
    assert_eq!(s.metadata.at_rule_name.as_deref(), Some("supports"));
}

// ── Complex selector tests ─────────────────────────────────────

#[test]
fn simple_media() {
    let items = parse_and_extract("@media (max-width: 600px) { .box { display: block; } }");
    let m = find_by_name(&items, "@media (max-width: 600px)");
    assert_eq!(m.kind, SymbolKind::Module);
}
