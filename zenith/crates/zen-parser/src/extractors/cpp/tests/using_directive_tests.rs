use super::*;

// ════════════════════════════════════════════════════════════════
// 35. Using directive tests
// ════════════════════════════════════════════════════════════════

#[test]
fn using_directive_std() {
    let items = fixture_items();
    let ud = items.iter().find(|i| {
        i.kind == SymbolKind::Module
            && i.metadata
                .attributes
                .contains(&"using_directive".to_string())
    });
    assert!(ud.is_some(), "using namespace std should be extracted");
}

#[test]
fn using_directive_name_is_std() {
    let items = fixture_items();
    let ud = items
        .iter()
        .find(|i| {
            i.kind == SymbolKind::Module
                && i.metadata
                    .attributes
                    .contains(&"using_directive".to_string())
        })
        .expect("using directive should exist");
    assert!(
        ud.name.contains("std"),
        "using directive name should contain 'std', got {:?}",
        ud.name
    );
}

#[test]
fn minimal_using_directive() {
    let items = parse_and_extract("using namespace std;");
    let ud = items.iter().find(|i| {
        i.metadata
            .attributes
            .contains(&"using_directive".to_string())
    });
    assert!(ud.is_some(), "using namespace std should be extracted");
}
