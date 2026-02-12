use super::*;

#[test]
fn module_level_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let max_retries = find_by_name(&items, "MAX_RETRIES");
    assert_eq!(max_retries.kind, SymbolKind::Const);
    // MAX_RETRIES is in __all__, so it gets Export visibility
    assert_eq!(max_retries.visibility, Visibility::Export);
    assert_eq!(max_retries.metadata.return_type.as_deref(), Some("int"),);
}

#[test]
fn module_level_float_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let timeout = find_by_name(&items, "DEFAULT_TIMEOUT");
    assert_eq!(timeout.kind, SymbolKind::Const);
    assert_eq!(timeout.metadata.return_type.as_deref(), Some("float"),);
}

#[test]
fn private_module_const_visibility() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let cache = find_by_name(&items, "_internal_cache");
    assert_eq!(cache.kind, SymbolKind::Const);
    assert_eq!(cache.visibility, Visibility::Protected);
}

#[test]
fn module_docstring_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let module = find_by_name(&items, "<module>");
    assert_eq!(module.kind, SymbolKind::Module);
    assert!(
        module.doc_comment.contains("Module docstring"),
        "doc: {:?}",
        module.doc_comment
    );
}

#[test]
fn dunder_version_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let version = find_by_name(&items, "__version__");
    assert_eq!(version.kind, SymbolKind::Const);
    assert_eq!(version.visibility, Visibility::Public);
}

#[test]
fn untyped_const_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let version = find_by_name(&items, "VERSION");
    assert_eq!(version.kind, SymbolKind::Const);
    let debug = find_by_name(&items, "DEBUG");
    assert_eq!(debug.kind, SymbolKind::Const);
}

#[test]
fn type_alias_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let json = find_by_name(&items, "JsonValue");
    assert_eq!(json.kind, SymbolKind::TypeAlias);
}

// ── Class feature tests ────────────────────────────────────────

#[test]
fn class_signature_format() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let base = find_by_name(&items, "BaseProcessor");
    assert!(
        base.signature.starts_with("class BaseProcessor"),
        "sig: {:?}",
        base.signature
    );
}

// ── Total item count sanity ────────────────────────────────────
