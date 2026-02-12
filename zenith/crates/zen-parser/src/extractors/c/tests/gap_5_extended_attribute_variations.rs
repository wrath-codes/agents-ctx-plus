use super::*;

// ── Gap 5 extended: __attribute__ variations ──────────────────

#[test]
fn gcc_attribute_deprecated() {
    let items = parse_and_extract("__attribute__((deprecated)) int old_api(void);");
    assert_eq!(items.len(), 1);
    assert!(
        items[0]
            .metadata
            .attributes
            .iter()
            .any(|a| a.contains("deprecated")),
        "should contain deprecated: {:?}",
        items[0].metadata.attributes
    );
}

#[test]
fn gcc_attribute_preserved_text() {
    let items = parse_and_extract("__attribute__((unused)) static int x = 0;");
    assert!(
        items[0]
            .metadata
            .attributes
            .iter()
            .any(|a| a.contains("__attribute__")),
        "should preserve __attribute__ text: {:?}",
        items[0].metadata.attributes
    );
}
