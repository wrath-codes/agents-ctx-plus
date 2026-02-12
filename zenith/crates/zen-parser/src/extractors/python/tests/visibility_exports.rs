use super::*;

#[test]
fn private_function_visibility() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let private = find_by_name(&items, "_private_helper");
    assert_eq!(private.visibility, Visibility::Protected);
}

#[test]
fn dunder_methods_are_public() {
    // __init__, __len__, etc. should be Public, not Private
    let vis = super::python_visibility("__init__");
    assert_eq!(vis, Visibility::Public);
    let vis = super::python_visibility("__len__");
    assert_eq!(vis, Visibility::Public);
}

#[test]
fn name_mangled_is_private() {
    let vis = super::python_visibility("__private_method");
    assert_eq!(vis, Visibility::Private);
}

#[test]
fn single_underscore_is_protected() {
    let vis = super::python_visibility("_protected_method");
    assert_eq!(vis, Visibility::Protected);
}

#[test]
fn regular_name_is_public() {
    let vis = super::python_visibility("public_method");
    assert_eq!(vis, Visibility::Public);
}

#[test]
fn all_exports_applied() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    // Items in __all__ should have Export visibility
    let base = find_by_name(&items, "BaseProcessor");
    assert_eq!(base.visibility, Visibility::Export);
    let config = find_by_name(&items, "Config");
    assert_eq!(config.visibility, Visibility::Export);
    let fetch = find_by_name(&items, "fetch_data");
    assert_eq!(fetch.visibility, Visibility::Export);
    let status = find_by_name(&items, "Status");
    assert_eq!(status.visibility, Visibility::Export);
}

#[test]
fn non_exported_stays_public() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    // Color is not in __all__, should stay Public
    let color = find_by_name(&items, "Color");
    assert_eq!(color.visibility, Visibility::Public);
}

// ── Module-level features ──────────────────────────────────────

#[test]
fn visibility_all_cases() {
    assert_eq!(super::python_visibility("foo"), Visibility::Public);
    assert_eq!(super::python_visibility("_foo"), Visibility::Protected);
    assert_eq!(super::python_visibility("__foo"), Visibility::Private);
    assert_eq!(super::python_visibility("__foo__"), Visibility::Public);
    assert_eq!(super::python_visibility("__init__"), Visibility::Public);
    assert_eq!(super::python_visibility("__version__"), Visibility::Public);
}

// ── Unit tests for parse_numpy_style ────────────────────────────
