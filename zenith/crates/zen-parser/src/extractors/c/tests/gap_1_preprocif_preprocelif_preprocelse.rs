use super::*;

// ── Gap 1: #if / #elif / #else ────────────────────────────────

#[test]
fn preproc_if_extracted() {
    let items = parse_and_extract("#if __STDC_VERSION__ >= 201112L\nint c11 = 1;\n#endif\n");
    let if_item = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"#if".to_string()))
        .expect("should find #if");
    assert_eq!(if_item.kind, SymbolKind::Macro);
}

#[test]
fn preproc_if_contains_condition() {
    let items = parse_and_extract("#if __STDC_VERSION__ >= 201112L\nint c11 = 1;\n#endif\n");
    let if_item = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"#if".to_string()))
        .expect("should find #if");
    assert!(
        if_item.name.contains("__STDC_VERSION__"),
        "should have condition in name: {:?}",
        if_item.name
    );
}

#[test]
fn preproc_if_children_extracted() {
    let items = parse_and_extract("#if __STDC_VERSION__ >= 201112L\nint c11 = 1;\n#endif\n");
    let c11 = find_by_name(&items, "c11");
    assert_eq!(c11.kind, SymbolKind::Static);
}

#[test]
fn preproc_elif_extracted() {
    let items = parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
    let elif = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"#elif".to_string()))
        .expect("should find #elif");
    assert_eq!(elif.kind, SymbolKind::Macro);
}

#[test]
fn preproc_elif_children_extracted() {
    let items = parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
    let b = find_by_name(&items, "b");
    assert_eq!(b.kind, SymbolKind::Static);
}

#[test]
fn preproc_else_children_extracted() {
    let items = parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
    let c = find_by_name(&items, "c");
    assert_eq!(c.kind, SymbolKind::Static);
}

#[test]
fn preproc_if_all_branches_have_items() {
    let items = parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
    // Should have: #if macro, a, #elif macro, b, c = 5 items
    assert!(
        items.len() >= 5,
        "expected at least 5 items, got {}",
        items.len()
    );
}

#[test]
fn fixture_has_preproc_if() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let if_items: Vec<_> = items
        .iter()
        .filter(|i| i.metadata.attributes.contains(&"#if".to_string()))
        .collect();
    assert!(
        if_items.len() >= 2,
        "expected at least 2 #if items, got {}",
        if_items.len()
    );
}

#[test]
fn fixture_c11_available() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let c11 = find_by_name(&items, "c11_available");
    assert_eq!(c11.kind, SymbolKind::Static);
}
