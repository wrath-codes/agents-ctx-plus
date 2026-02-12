use super::*;

#[test]
fn doc_sections_errors_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let transform = find_by_name(&items, "transform");
    assert!(
        transform.metadata.doc_sections.errors.is_some(),
        "should have # Errors section"
    );
}

#[test]
fn error_type_by_name_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let my_error = find_by_name(&items, "MyError");
    assert_eq!(my_error.kind, SymbolKind::Enum);
    assert!(
        my_error.metadata.is_error_type,
        "MyError should be detected as error type"
    );
}

#[test]
fn error_enum_variants_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let my_error = find_by_name(&items, "MyError");
    assert!(
        my_error
            .metadata
            .variants
            .iter()
            .any(|v| v.starts_with("Io")),
        "variants: {:?}",
        my_error.metadata.variants
    );
    assert!(
        my_error.metadata.variants.iter().any(|v| v == "NotFound"),
        "variants: {:?}",
        my_error.metadata.variants
    );
}

#[test]
fn pyo3_function_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let py_add = find_by_name(&items, "py_add");
    assert!(py_add.metadata.is_pyo3, "py_add should be detected as PyO3");
}

// ── Extended fixture tests ─────────────────────────────────────

#[test]
fn cfg_attribute_preserved() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "serde_only");
    assert!(
        f.metadata.attributes.iter().any(|a| a.starts_with("cfg(")),
        "should have cfg attr: {:?}",
        f.metadata.attributes
    );
}

#[test]
fn deprecated_attribute_preserved() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "old_api");
    assert!(
        f.metadata
            .attributes
            .iter()
            .any(|a| a.starts_with("deprecated")),
        "should have deprecated attr: {:?}",
        f.metadata.attributes
    );
}

#[test]
fn must_use_attribute_preserved() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "important_result");
    assert!(
        f.metadata.attributes.iter().any(|a| a == "must_use"),
        "should have must_use attr: {:?}",
        f.metadata.attributes
    );
}

#[test]
fn doc_hidden_attribute_preserved() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "internal_only");
    assert!(
        f.metadata
            .attributes
            .iter()
            .any(|a| a.contains("doc(hidden)")),
        "should have doc(hidden) attr: {:?}",
        f.metadata.attributes
    );
}

#[test]
fn block_doc_comment_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let f = find_by_name(&items, "block_documented");
    assert!(
        f.doc_comment.contains("Block documented"),
        "doc_comment: {:?}",
        f.doc_comment
    );
}
