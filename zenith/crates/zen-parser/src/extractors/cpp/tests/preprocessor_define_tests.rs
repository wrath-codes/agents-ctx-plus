use super::*;

// ════════════════════════════════════════════════════════════════
// 19. Preprocessor define tests
// ════════════════════════════════════════════════════════════════

#[test]
fn define_max_size() {
    let items = fixture_items();
    let ms = find_by_name(&items, "MAX_SIZE");
    assert_eq!(ms.kind, SymbolKind::Const, "MAX_SIZE should be Const");
}

#[test]
fn define_app_version() {
    let items = fixture_items();
    let av = find_by_name(&items, "APP_VERSION");
    assert_eq!(av.kind, SymbolKind::Const, "APP_VERSION should be Const");
}
