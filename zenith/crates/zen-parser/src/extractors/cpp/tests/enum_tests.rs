use super::*;

// ════════════════════════════════════════════════════════════════
// 6. Enum tests
// ════════════════════════════════════════════════════════════════

#[test]
fn enum_color_exists() {
    let items = fixture_items();
    let color = find_by_name(&items, "Color");
    assert_eq!(color.kind, SymbolKind::Enum);
}

#[test]
fn enum_color_is_scoped() {
    let items = fixture_items();
    let color = find_by_name(&items, "Color");
    assert!(
        color
            .metadata
            .attributes
            .contains(&"scoped_enum".to_string()),
        "Color should be a scoped enum, got {:?}",
        color.metadata.attributes
    );
}

#[test]
fn enum_color_has_variants() {
    let items = fixture_items();
    let color = find_by_name(&items, "Color");
    assert!(
        color.metadata.variants.contains(&"Red".to_string()),
        "Color should have Red variant"
    );
    assert!(
        color.metadata.variants.contains(&"Green".to_string()),
        "Color should have Green variant"
    );
    assert!(
        color.metadata.variants.contains(&"Blue".to_string()),
        "Color should have Blue variant"
    );
}

#[test]
fn enum_status_code_scoped() {
    let items = fixture_items();
    let sc = find_by_name(&items, "StatusCode");
    assert_eq!(sc.kind, SymbolKind::Enum);
    assert!(
        sc.metadata.attributes.contains(&"scoped_enum".to_string()),
        "StatusCode should be a scoped enum"
    );
}

#[test]
fn enum_status_code_has_variants() {
    let items = fixture_items();
    let sc = find_by_name(&items, "StatusCode");
    assert!(
        sc.metadata.variants.contains(&"OK".to_string()),
        "StatusCode should have OK variant"
    );
    assert!(
        sc.metadata.variants.contains(&"NotFound".to_string()),
        "StatusCode should have NotFound variant"
    );
    assert!(
        sc.metadata.variants.contains(&"InternalError".to_string()),
        "StatusCode should have InternalError variant"
    );
}

#[test]
fn enum_log_level_unscoped() {
    let items = fixture_items();
    let ll = find_by_name(&items, "LogLevel");
    assert_eq!(ll.kind, SymbolKind::Enum);
    assert!(
        !ll.metadata.attributes.contains(&"scoped_enum".to_string()),
        "LogLevel should NOT be scoped"
    );
}

#[test]
fn enum_log_level_has_variants() {
    let items = fixture_items();
    let ll = find_by_name(&items, "LogLevel");
    assert!(
        ll.metadata.variants.len() >= 4,
        "LogLevel should have 4+ variants, got {}",
        ll.metadata.variants.len()
    );
}

#[test]
fn enum_color_has_doc_comment() {
    let items = fixture_items();
    let color = find_by_name(&items, "Color");
    assert!(
        !color.doc_comment.is_empty(),
        "Color should have a doc comment"
    );
}
