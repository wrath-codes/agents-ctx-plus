use super::*;

#[test]
fn exception_subclass_is_error_type() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let proc_err = find_by_name(&items, "ProcessingError");
    assert!(
        proc_err.metadata.is_error_type,
        "ProcessingError(Exception) should be error type"
    );
    assert!(
        proc_err
            .metadata
            .base_classes
            .contains(&"Exception".to_string()),
        "base_classes: {:?}",
        proc_err.metadata.base_classes
    );
}

#[test]
fn value_error_subclass_is_error_type() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let val_err = find_by_name(&items, "ValidationError");
    assert!(
        val_err.metadata.is_error_type,
        "ValidationError(ValueError) should be error type"
    );
}

#[test]
fn exception_docstring_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let proc_err = find_by_name(&items, "ProcessingError");
    assert!(
        proc_err.doc_comment.contains("processing fails"),
        "doc: {:?}",
        proc_err.doc_comment
    );
}

#[test]
fn error_type_by_exception_name() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let proc_err = find_by_name(&items, "ProcessingError");
    assert!(proc_err.metadata.is_error_type);
    let val_err = find_by_name(&items, "ValidationError");
    assert!(val_err.metadata.is_error_type);
}

// ── Signature tests ────────────────────────────────────────────
