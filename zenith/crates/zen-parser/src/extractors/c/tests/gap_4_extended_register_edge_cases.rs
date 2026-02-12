use super::*;

// ── Gap 4 extended: register edge cases ───────────────────────

#[test]
fn register_with_init() {
    let items = parse_and_extract("register int r = 42;");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"register".to_string())
    );
}

#[test]
fn register_not_static_visibility() {
    let items = parse_and_extract("register int r;");
    assert_eq!(items[0].visibility, Visibility::Public);
}
