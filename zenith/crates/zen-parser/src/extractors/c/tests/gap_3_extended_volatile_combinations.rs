use super::*;

// ── Gap 3 extended: volatile combinations ─────────────────────

#[test]
fn volatile_static_combined() {
    let items = parse_and_extract("static volatile int flag = 0;");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .contains(&"volatile".to_string())
    );
    assert!(items[0].metadata.attributes.contains(&"static".to_string()));
    assert_eq!(items[0].visibility, Visibility::Private);
}

#[test]
fn volatile_return_type_in_variable() {
    let items = parse_and_extract("volatile int reg;");
    assert!(
        items[0]
            .metadata
            .return_type
            .as_deref()
            .is_some_and(|rt| rt.contains("int")),
        "return_type should include int: {:?}",
        items[0].metadata.return_type
    );
}
