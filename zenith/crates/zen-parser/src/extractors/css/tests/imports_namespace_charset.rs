use super::*;

#[test]
fn import_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let imports = find_all_by_at_rule(&items, "import");
    assert_eq!(imports.len(), 2, "should find 2 @import statements");
}

#[test]
fn import_url_as_name() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "reset.css");
    assert_eq!(i.kind, SymbolKind::Module);
    assert_eq!(i.metadata.at_rule_name.as_deref(), Some("import"));
}

#[test]
fn import_nested_url() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let i = find_by_name(&items, "theme/dark.css");
    assert_eq!(i.kind, SymbolKind::Module);
}

// ── Charset tests ──────────────────────────────────────────────

#[test]
fn charset_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "@charset UTF-8");
    assert_eq!(c.kind, SymbolKind::Const);
    assert_eq!(c.metadata.at_rule_name.as_deref(), Some("charset"));
}

// ── Namespace tests ────────────────────────────────────────────

#[test]
fn namespace_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.css");
    let items = parse_and_extract(source);
    let n = find_by_name(&items, "@namespace svg");
    assert_eq!(n.kind, SymbolKind::Module);
    assert_eq!(n.metadata.at_rule_name.as_deref(), Some("namespace"));
}

// ── Custom property tests ──────────────────────────────────────

#[test]
fn simple_import() {
    let items = parse_and_extract("@import url('base.css');");
    let i = find_by_name(&items, "base.css");
    assert_eq!(i.kind, SymbolKind::Module);
}

#[test]
fn simple_charset() {
    let items = parse_and_extract("@charset \"UTF-8\";");
    let c = find_by_name(&items, "@charset UTF-8");
    assert_eq!(c.kind, SymbolKind::Const);
}
