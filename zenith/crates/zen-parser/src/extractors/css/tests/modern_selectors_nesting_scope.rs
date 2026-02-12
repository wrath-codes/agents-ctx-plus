use super::*;

#[test]
fn is_pseudo_function() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, ":is(h1, h2, h3)");
    assert_eq!(i.kind, SymbolKind::Class);
}

#[test]
fn where_pseudo_function() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let w = find_by_name(&items, ":where(.card, .panel)");
    assert_eq!(w.kind, SymbolKind::Class);
}

#[test]
fn has_pseudo_function() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let h = find_by_name(&items, "article:has(> img)");
    assert_eq!(h.kind, SymbolKind::Class);
}

#[test]
fn not_pseudo_function() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let n = find_by_name(&items, ":not(.active)");
    assert_eq!(n.kind, SymbolKind::Class);
}

// ── Native CSS nesting tests ───────────────────────────────────

#[test]
fn nesting_parent_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let n = find_by_name(&items, ".nav");
    assert_eq!(n.kind, SymbolKind::Class);
}

#[test]
fn nesting_inline() {
    let items = parse_and_extract(".card { color: black; & .title { font-size: 2rem; } }");
    let c = find_by_name(&items, ".card");
    assert_eq!(c.kind, SymbolKind::Class);
    // Nested rule should also be extracted (it's a rule_set inside block)
}

// ── @page tests ────────────────────────────────────────────────

#[test]
fn scope_with_to_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "@scope (.card) to (.card__body)");
    assert_eq!(s.kind, SymbolKind::Module);
    assert_eq!(s.metadata.at_rule_name.as_deref(), Some("scope"));
}

#[test]
fn scope_without_to_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let s = find_by_name(&items, "@scope (.hero)");
    assert_eq!(s.kind, SymbolKind::Module);
    assert_eq!(s.metadata.at_rule_name.as_deref(), Some("scope"));
}

#[test]
fn scope_count() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let scopes = find_all_by_at_rule(&items, "scope");
    assert_eq!(scopes.len(), 2, "should find 2 @scope statements");
}

#[test]
fn scope_nested_rules() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    // Rules inside @scope should have parent context
    assert!(
        items
            .iter()
            .any(|i| i.name.contains("@.card") || i.name.contains("@.hero")),
        "should find nested rules inside @scope"
    );
}

#[test]
fn scope_inline() {
    let items = parse_and_extract("@scope (.panel) { h2 { font-size: 1.5rem; } }");
    let s = find_by_name(&items, "@scope (.panel)");
    assert_eq!(s.kind, SymbolKind::Module);
}

// ── @starting-style tests ──────────────────────────────────────

#[test]
fn inline_scope() {
    let items = parse_and_extract("@scope (.wrapper) to (.inner) { div { padding: 1rem; } }");
    let s = find_by_name(&items, "@scope (.wrapper) to (.inner)");
    assert_eq!(s.kind, SymbolKind::Module);
}
