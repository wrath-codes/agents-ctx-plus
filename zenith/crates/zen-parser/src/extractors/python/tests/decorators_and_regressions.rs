use super::*;

#[test]
fn no_duplicate_decorated_items() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    // Config has @dataclass — should appear exactly once
    let config_count = items.iter().filter(|i| i.name == "Config").count();
    assert_eq!(config_count, 1, "decorated class should not be duplicated");
}

#[test]
fn no_methods_as_top_level_functions() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    // "process" is a method of BaseProcessor, not a top-level function
    let process_items: Vec<_> = items.iter().filter(|i| i.name == "process").collect();
    assert!(
        process_items.is_empty(),
        "methods should not be extracted as top-level items: {:?}",
        process_items.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

#[test]
fn no_nested_functions_as_top_level() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    // inner_function is nested inside outer_function
    let inner = items.iter().find(|i| i.name == "inner_function");
    assert!(
        inner.is_none(),
        "nested functions should not be top-level items"
    );
}

#[test]
fn no_nested_classes_as_top_level() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    // Inner is nested inside Outer
    let inner = items.iter().find(|i| i.name == "Inner");
    assert!(
        inner.is_none(),
        "nested classes should not be top-level items"
    );
}

// ── Visibility tests ───────────────────────────────────────────

#[test]
fn decorator_matches_dotted_path() {
    let decorators = vec!["dataclasses.dataclass".to_string()];
    assert!(super::decorator_matches(&decorators, "dataclass"));
}

#[test]
fn decorator_matches_with_args() {
    let decorators = vec!["dataclass(frozen=True)".to_string()];
    assert!(super::decorator_matches(&decorators, "dataclass"));
}

#[test]
fn decorator_matches_exact() {
    let decorators = vec!["staticmethod".to_string()];
    assert!(super::decorator_matches(&decorators, "staticmethod"));
}

// ── Property tests ─────────────────────────────────────────────
