use super::*;

// ── Hook detection ─────────────────────────────────────────────

#[test]
fn use_theme_is_hook() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let hook = find_by_name(&items, "useTheme");
    assert!(hook.metadata.is_hook);
    assert!(!hook.metadata.is_component);
}

#[test]
fn use_theme_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let hook = find_by_name(&items, "useTheme");
    assert!(
        hook.doc_comment.contains("Custom hook for theme access"),
        "doc: {:?}",
        hook.doc_comment
    );
}

#[test]
fn use_theme_hooks_used() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let hook = find_by_name(&items, "useTheme");
    assert!(
        hook.metadata.hooks_used.contains(&"useContext".to_string()),
        "hooks: {:?}",
        hook.metadata.hooks_used
    );
}

#[test]
fn use_fetch_is_hook() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let hook = find_by_name(&items, "useFetch");
    assert!(hook.metadata.is_hook);
    assert!(!hook.metadata.is_component);
}

#[test]
fn use_fetch_hooks_used() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let hook = find_by_name(&items, "useFetch");
    assert!(
        hook.metadata.hooks_used.contains(&"useState".to_string()),
        "hooks: {:?}",
        hook.metadata.hooks_used
    );
    assert!(
        hook.metadata.hooks_used.contains(&"useEffect".to_string()),
        "hooks: {:?}",
        hook.metadata.hooks_used
    );
}

#[test]
fn use_fetch_has_type_params() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let hook = find_by_name(&items, "useFetch");
    assert!(
        hook.metadata
            .type_parameters
            .as_deref()
            .is_some_and(|t| t.contains('T')),
        "type_params: {:?}",
        hook.metadata.type_parameters
    );
}
