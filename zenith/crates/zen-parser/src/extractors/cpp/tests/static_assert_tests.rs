use super::*;

// ════════════════════════════════════════════════════════════════
// 10. Static assert tests
// ════════════════════════════════════════════════════════════════

#[test]
fn static_assert_extracted() {
    let items = fixture_items();
    let asserts: Vec<_> = items
        .iter()
        .filter(|i| i.name == "static_assert" && i.kind == SymbolKind::Macro)
        .collect();
    assert!(
        asserts.len() >= 3,
        "expected 3+ static_asserts, got {}",
        asserts.len()
    );
}

#[test]
fn static_assert_has_attribute() {
    let items = fixture_items();
    let sa = items
        .iter()
        .find(|i| i.name == "static_assert" && i.kind == SymbolKind::Macro)
        .expect("static_assert should exist");
    assert!(
        sa.metadata
            .attributes
            .contains(&"static_assert".to_string()),
        "static_assert should have static_assert attribute"
    );
}

#[test]
fn static_assert_has_signature() {
    let items = fixture_items();
    let sa = items
        .iter()
        .find(|i| i.name == "static_assert" && i.kind == SymbolKind::Macro)
        .expect("static_assert should exist");
    assert!(
        !sa.signature.is_empty(),
        "static_assert should have a signature"
    );
}
