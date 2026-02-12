use super::*;

// ════════════════════════════════════════════════════════════════
// 2. Include tests
// ════════════════════════════════════════════════════════════════

#[test]
fn includes_extracted() {
    let items = fixture_items();
    let includes: Vec<_> = items
        .iter()
        .filter(|i| i.kind == SymbolKind::Module && i.name.starts_with('<'))
        .collect();
    assert!(
        includes.len() >= 9,
        "expected 9+ includes, got {}",
        includes.len()
    );
}

#[test]
fn include_iostream_present() {
    let items = fixture_items();
    assert!(
        items.iter().any(|i| i.name.contains("iostream")),
        "expected <iostream> include"
    );
}

#[test]
fn include_concepts_present() {
    let items = fixture_items();
    assert!(
        items.iter().any(|i| i.name.contains("concepts")),
        "expected <concepts> include"
    );
}
