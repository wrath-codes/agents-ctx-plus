use super::*;

// ── Gap 2 extended: multi-variable edge cases ─────────────────

#[test]
fn multi_var_const_all_extracted() {
    let items = parse_and_extract("const int CA = 1, CB = 2;");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].kind, SymbolKind::Const);
    assert_eq!(items[1].kind, SymbolKind::Const);
}

#[test]
fn multi_var_two_items() {
    let items = parse_and_extract("float p = 1.0, q = 2.0;");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "p");
    assert_eq!(items[1].name, "q");
}

#[test]
fn multi_var_static_visibility() {
    let items = parse_and_extract("static int sa = 1, sb = 2;");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].visibility, Visibility::Private);
    assert_eq!(items[1].visibility, Visibility::Private);
}
