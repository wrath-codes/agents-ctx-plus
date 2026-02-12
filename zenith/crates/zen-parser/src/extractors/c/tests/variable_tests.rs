use super::*;

// ── Variable tests ────────────────────────────────────────────

#[test]
fn variable_global_counter() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let gc = find_by_name(&items, "global_counter");
    assert_eq!(gc.kind, SymbolKind::Static);
    assert_eq!(gc.visibility, Visibility::Public);
}

#[test]
fn variable_internal_state_static() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let is_ = find_by_name(&items, "internal_state");
    assert_eq!(is_.kind, SymbolKind::Static);
    assert_eq!(is_.visibility, Visibility::Private);
    assert!(
        is_.metadata.attributes.contains(&"static".to_string()),
        "should have static attr"
    );
}

#[test]
fn variable_shared_value_extern() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let sv = find_by_name(&items, "shared_value");
    assert_eq!(sv.visibility, Visibility::Public);
    assert!(
        sv.metadata.attributes.contains(&"extern".to_string()),
        "should have extern attr"
    );
}

#[test]
fn constant_max_items() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let mi = find_by_name(&items, "MAX_ITEMS");
    assert_eq!(mi.kind, SymbolKind::Const);
    assert!(
        mi.metadata.attributes.contains(&"const".to_string()),
        "should have const attr"
    );
}

#[test]
fn constant_default_timeout() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let dt = find_by_name(&items, "DEFAULT_TIMEOUT_MS");
    assert_eq!(dt.kind, SymbolKind::Const);
}

#[test]
fn variable_build_tag_static_const() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let bt = find_by_name(&items, "BUILD_TAG");
    assert_eq!(bt.kind, SymbolKind::Const);
    assert_eq!(bt.visibility, Visibility::Private);
    assert!(
        bt.metadata.attributes.contains(&"static".to_string()),
        "should have static"
    );
    assert!(
        bt.metadata.attributes.contains(&"const".to_string()),
        "should have const"
    );
}
