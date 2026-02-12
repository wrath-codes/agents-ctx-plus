use super::*;

// ── Pointer-to-pointer ────────────────────────────────────────

#[test]
fn pointer_to_pointer_extracted() {
    let items = parse_and_extract("char **envp;");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "envp");
}

#[test]
fn fixture_environment_ptr_ptr() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let env = find_by_name(&items, "environment");
    assert_eq!(env.kind, SymbolKind::Static);
}
