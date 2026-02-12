use super::*;

// ════════════════════════════════════════════════════════════════
// 5. Namespace tests
// ════════════════════════════════════════════════════════════════

#[test]
fn namespace_math_exists() {
    let items = fixture_items();
    let m = find_by_name(&items, "math");
    assert_eq!(m.kind, SymbolKind::Module);
    assert!(
        m.metadata.attributes.contains(&"namespace".to_string()),
        "math should have namespace attribute"
    );
}

#[test]
fn namespace_math_is_public() {
    let items = fixture_items();
    let m = find_by_name(&items, "math");
    assert_eq!(m.visibility, Visibility::Public);
}

#[test]
fn namespace_nested_utils_string() {
    let items = fixture_items();
    let ns = items.iter().find(|i| {
        i.kind == SymbolKind::Module
            && i.metadata.attributes.contains(&"namespace".to_string())
            && (i.name.contains("utils") || i.name.contains("string"))
    });
    assert!(ns.is_some(), "nested namespace utils::string should exist");
}

#[test]
fn namespace_anonymous_exists() {
    let items = fixture_items();
    let anon = find_by_name(&items, "(anonymous)");
    assert_eq!(anon.kind, SymbolKind::Module);
    assert_eq!(anon.visibility, Visibility::Private);
}

#[test]
fn namespace_math_contains_abs() {
    let items = fixture_items();
    assert!(
        items
            .iter()
            .any(|i| i.name == "abs" && i.kind == SymbolKind::Function),
        "abs function in math namespace should exist"
    );
}

#[test]
fn namespace_math_contains_square() {
    let items = fixture_items();
    assert!(
        items
            .iter()
            .any(|i| i.name == "square" && i.kind == SymbolKind::Function),
        "square function in math namespace should exist"
    );
}

#[test]
fn namespace_math_contains_point_struct() {
    let items = fixture_items();
    assert!(
        items
            .iter()
            .any(|i| i.name == "Point" && i.kind == SymbolKind::Struct),
        "Point struct in math namespace should exist"
    );
}

#[test]
fn namespace_utils_string_contains_trim() {
    let items = fixture_items();
    assert!(
        items
            .iter()
            .any(|i| i.name == "trim" && i.kind == SymbolKind::Function),
        "trim function in utils::string namespace should exist"
    );
}
