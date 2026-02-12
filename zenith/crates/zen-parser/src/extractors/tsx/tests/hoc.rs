use super::*;

// ── HOC detection ──────────────────────────────────────────────

#[test]
fn with_loading_is_hoc() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let hoc = find_by_name(&items, "withLoading");
    assert!(hoc.metadata.is_hoc);
    assert!(!hoc.metadata.is_component, "HOC should not be a component");
}

#[test]
fn with_loading_not_hook() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let hoc = find_by_name(&items, "withLoading");
    assert!(!hoc.metadata.is_hook);
}
