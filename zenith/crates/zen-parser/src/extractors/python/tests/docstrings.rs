use super::*;

#[test]
fn google_style_args_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let base = find_by_name(&items, "BaseProcessor");
    let process_method = base.metadata.methods.iter().find(|m| *m == "process");
    assert!(process_method.is_some(), "should have process method");
}

#[test]
fn sphinx_style_docstring_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let transform = find_by_name(&items, "transform");
    assert!(
        !transform.metadata.doc_sections.args.is_empty(),
        "sphinx :param should be parsed: {:?}",
        transform.metadata.doc_sections.args
    );
    assert!(
        transform.metadata.doc_sections.returns.is_some(),
        "sphinx :returns: should be parsed"
    );
    assert!(
        !transform.metadata.doc_sections.raises.is_empty(),
        "sphinx :raises: should be parsed: {:?}",
        transform.metadata.doc_sections.raises
    );
}

#[test]
fn numpy_style_args_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let numpy = find_by_name(&items, "numpy_documented");
    assert!(
        !numpy.metadata.doc_sections.args.is_empty(),
        "NumPy args should be parsed: {:?}",
        numpy.metadata.doc_sections.args
    );
    assert!(
        numpy.metadata.doc_sections.args.contains_key("x"),
        "should have 'x' param: {:?}",
        numpy.metadata.doc_sections.args
    );
}

#[test]
fn numpy_style_returns_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let numpy = find_by_name(&items, "numpy_documented");
    assert!(
        numpy.metadata.doc_sections.returns.is_some(),
        "NumPy Returns should be parsed"
    );
}

#[test]
fn numpy_style_raises_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let numpy = find_by_name(&items, "numpy_documented");
    assert!(
        !numpy.metadata.doc_sections.raises.is_empty(),
        "NumPy Raises should be parsed: {:?}",
        numpy.metadata.doc_sections.raises
    );
}

#[test]
fn numpy_style_examples_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let numpy = find_by_name(&items, "numpy_documented");
    assert!(
        numpy.metadata.doc_sections.examples.is_some(),
        "NumPy Examples should be parsed"
    );
}

#[test]
fn numpy_style_notes_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let numpy = find_by_name(&items, "numpy_documented");
    assert!(
        numpy.metadata.doc_sections.notes.is_some(),
        "NumPy Notes should be parsed"
    );
}

// ── Decorator semantics tests ──────────────────────────────────

#[test]
fn numpy_parse_basic() {
    let doc = "Summary.\n\nParameters\n----------\nx : float\n    The x.\ny : int\n    The y.\n\nReturns\n-------\nfloat\n    The result.";
    let sections = super::parse_numpy_style(doc);
    assert!(sections.args.contains_key("x"), "args: {:?}", sections.args);
    assert!(sections.args.contains_key("y"), "args: {:?}", sections.args);
    assert!(sections.returns.is_some());
}

// ── Error type tests ───────────────────────────────────────────
