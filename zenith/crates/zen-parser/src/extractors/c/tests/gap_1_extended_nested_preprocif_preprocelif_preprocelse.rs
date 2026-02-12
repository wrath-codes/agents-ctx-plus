use super::*;

// ── Gap 1 extended: nested #if / #elif / #else ──────────────────

#[test]
fn preproc_if_nested_ifdef_inside() {
    let src = "#if A\n#ifdef B\nint inner = 1;\n#endif\n#endif\n";
    let items = parse_and_extract(src);
    let inner = find_by_name(&items, "inner");
    assert_eq!(inner.kind, SymbolKind::Static);
}

#[test]
fn preproc_if_with_defined() {
    let src = "#if defined(__GNUC__)\nint gcc = 1;\n#endif\n";
    let items = parse_and_extract(src);
    let gcc = find_by_name(&items, "gcc");
    assert_eq!(gcc.kind, SymbolKind::Static);
}

#[test]
fn preproc_elif_condition_name() {
    let items = parse_and_extract("#if X\nint a=1;\n#elif Y\nint b=2;\n#else\nint c=3;\n#endif\n");
    let elif = items
        .iter()
        .find(|i| i.metadata.attributes.contains(&"#elif".to_string()))
        .expect("should find #elif");
    assert!(
        elif.signature.contains("#elif"),
        "signature should contain #elif: {:?}",
        elif.signature
    );
}

#[test]
fn preproc_else_does_not_create_macro_item() {
    let items = parse_and_extract("#if X\nint a=1;\n#else\nint fallback=1;\n#endif\n");
    // #else has no condition so no macro item for it; just its children
    let fallback = find_by_name(&items, "fallback");
    assert_eq!(fallback.kind, SymbolKind::Static);
}

#[test]
fn preproc_if_struct_inside() {
    let src = "#if 1\nstruct IfStruct { int field; };\n#endif\n";
    let items = parse_and_extract(src);
    let s = find_by_name(&items, "IfStruct");
    assert_eq!(s.kind, SymbolKind::Struct);
    assert!(s.metadata.fields.contains(&"field".to_string()));
}
