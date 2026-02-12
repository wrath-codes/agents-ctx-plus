use super::*;

// ════════════════════════════════════════════════════════════════
// 31. Template instantiation tests
// ════════════════════════════════════════════════════════════════

#[test]
fn explicit_template_instantiation() {
    let items = fixture_items();
    let inst = items.iter().find(|i| {
        i.metadata
            .attributes
            .contains(&"explicit_instantiation".to_string())
    });
    assert!(
        inst.is_some(),
        "explicit template instantiation should be extracted"
    );
}

#[test]
fn explicit_template_instantiation_name() {
    let items = fixture_items();
    let inst = items
        .iter()
        .find(|i| {
            i.metadata
                .attributes
                .contains(&"explicit_instantiation".to_string())
        })
        .expect("instantiation should exist");
    assert!(
        inst.name.contains("Container"),
        "instantiation should reference Container, got {:?}",
        inst.name
    );
}
