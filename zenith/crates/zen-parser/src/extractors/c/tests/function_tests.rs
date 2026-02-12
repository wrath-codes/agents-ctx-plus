use super::*;

// ── Function tests ────────────────────────────────────────────

#[test]
fn function_add_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let funcs: Vec<_> = items
        .iter()
        .filter(|i| i.name == "add" && i.kind == SymbolKind::Function)
        .collect();
    // One prototype + one definition
    assert!(
        funcs.len() >= 2,
        "expected at least 2 'add' items (prototype + def), got {}",
        funcs.len()
    );
}

#[test]
fn function_add_has_params() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let add_def = items
        .iter()
        .find(|i| i.name == "add" && !i.metadata.attributes.contains(&"prototype".to_string()))
        .expect("should find add definition");
    assert_eq!(
        add_def.metadata.parameters.len(),
        2,
        "add should have 2 params: {:?}",
        add_def.metadata.parameters
    );
}

#[test]
fn function_add_return_type() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let add_def = items
        .iter()
        .find(|i| i.name == "add" && !i.metadata.attributes.contains(&"prototype".to_string()))
        .expect("should find add definition");
    assert_eq!(
        add_def.metadata.return_type.as_deref(),
        Some("int"),
        "add should return int"
    );
}

#[test]
fn function_add_doc_comment() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let add_def = items
        .iter()
        .find(|i| i.name == "add" && !i.metadata.attributes.contains(&"prototype".to_string()))
        .expect("should find add definition");
    assert!(
        add_def.doc_comment.contains("Add two integers"),
        "expected doc comment about adding, got: {:?}",
        add_def.doc_comment
    );
}

#[test]
fn function_clamp_is_static_inline() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let clamp = find_by_name(&items, "clamp_value");
    assert_eq!(clamp.kind, SymbolKind::Function);
    assert_eq!(clamp.visibility, Visibility::Private);
    assert!(
        clamp.metadata.attributes.contains(&"static".to_string()),
        "should have static attr: {:?}",
        clamp.metadata.attributes
    );
    assert!(
        clamp.metadata.attributes.contains(&"inline".to_string()),
        "should have inline attr: {:?}",
        clamp.metadata.attributes
    );
}

#[test]
fn function_multiply_is_extern() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let mul = find_by_name(&items, "multiply");
    assert_eq!(mul.kind, SymbolKind::Function);
    assert!(
        mul.metadata.attributes.contains(&"extern".to_string()),
        "should have extern attr: {:?}",
        mul.metadata.attributes
    );
}

#[test]
fn function_variadic_log() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let vlog = find_by_name(&items, "variadic_log");
    assert_eq!(vlog.kind, SymbolKind::Function);
    assert!(
        vlog.metadata.attributes.contains(&"variadic".to_string()),
        "should have variadic attr: {:?}",
        vlog.metadata.attributes
    );
}

#[test]
fn function_make_point_returns_struct() {
    let source = include_str!("../../../../tests/fixtures/sample.c");
    let items = parse_and_extract(source);
    let mp = find_by_name(&items, "make_point");
    assert_eq!(mp.kind, SymbolKind::Function);
    assert!(
        mp.metadata
            .return_type
            .as_deref()
            .is_some_and(|rt| rt.contains("Point")),
        "make_point should return struct Point: {:?}",
        mp.metadata.return_type
    );
}
