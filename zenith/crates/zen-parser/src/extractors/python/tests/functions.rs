use super::*;

#[test]
fn async_function_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let fetch = find_by_name(&items, "fetch_data");
    assert!(fetch.metadata.is_async);
    assert_eq!(fetch.kind, SymbolKind::Function);
}

#[test]
fn function_docstring_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let fetch = find_by_name(&items, "fetch_data");
    assert!(
        fetch.doc_comment.contains("Fetch data"),
        "doc: {:?}",
        fetch.doc_comment
    );
}

#[test]
fn function_return_type_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let fetch = find_by_name(&items, "fetch_data");
    assert_eq!(fetch.metadata.return_type.as_deref(), Some("bytes"));
}

#[test]
fn function_parameters_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let fetch = find_by_name(&items, "fetch_data");
    assert!(
        !fetch.metadata.parameters.is_empty(),
        "should have parameters"
    );
}

#[test]
fn generator_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let generator = find_by_name(&items, "generate_items");
    assert!(
        generator.metadata.is_generator,
        "should detect yield as generator"
    );
}

#[test]
fn google_style_yields_parsed() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let generator = find_by_name(&items, "generate_items");
    assert!(
        generator.metadata.doc_sections.yields.is_some(),
        "Yields: section should be parsed"
    );
}

#[test]
fn variadic_params_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let transform = find_by_name(&items, "transform");
    let params_str = transform.metadata.parameters.join(", ");
    assert!(
        params_str.contains("args"),
        "params: {:?}",
        transform.metadata.parameters
    );
    assert!(
        params_str.contains("kwargs"),
        "params: {:?}",
        transform.metadata.parameters
    );
}

#[test]
fn overload_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let overloads: Vec<_> = items.iter().filter(|i| i.name == "parse_input").collect();
    // Should have the overloaded versions
    assert!(
        overloads.len() >= 2,
        "should have overloaded parse_input: found {}",
        overloads.len()
    );
    let has_overload_attr = overloads
        .iter()
        .any(|i| i.metadata.attributes.contains(&"overload".to_string()));
    assert!(
        has_overload_attr,
        "at least one should have overload attribute"
    );
}

#[test]
fn context_manager_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let managed = find_by_name(&items, "managed_resource");
    assert!(
        managed
            .metadata
            .attributes
            .contains(&"contextmanager".to_string()),
        "attrs: {:?}",
        managed.metadata.attributes
    );
}

#[test]
fn multiple_decorators_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let multi = find_by_name(&items, "multi_decorated");
    assert!(
        multi.metadata.decorators.len() >= 2,
        "decorators: {:?}",
        multi.metadata.decorators
    );
}

#[test]
fn async_generator() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let ag = find_by_name(&items, "async_generate");
    assert!(ag.metadata.is_async, "should be async");
    assert!(ag.metadata.is_generator, "should be generator");
}

#[test]
fn generator_not_false_positive_on_nested() {
    // outer_function contains inner_function but no yield itself
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let outer = find_by_name(&items, "outer_function");
    assert!(
        !outer.metadata.is_generator,
        "outer_function should not be a generator (yield is only in nested)"
    );
}

#[test]
fn mixed_params_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let mixed = find_by_name(&items, "mixed_params");
    assert!(
        !mixed.metadata.parameters.is_empty(),
        "should have parameters"
    );
}

// ── Docstring parsing tests ────────────────────────────────────

#[test]
fn async_function_signature_prefix() {
    let source = include_str!("../../../../tests/fixtures/sample.py");
    let items = parse_and_extract(source);
    let fetch = find_by_name(&items, "fetch_data");
    assert!(
        fetch.signature.starts_with("async def"),
        "sig: {:?}",
        fetch.signature
    );
}
