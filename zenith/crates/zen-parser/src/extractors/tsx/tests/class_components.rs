use super::*;

// ── Class components ───────────────────────────────────────────

#[test]
fn error_boundary_is_class_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let eb = find_by_name(&items, "ErrorBoundary");
    assert_eq!(eb.kind, SymbolKind::Component);
    assert!(eb.metadata.is_class_component);
    assert!(eb.metadata.is_component);
}

#[test]
fn error_boundary_is_error_boundary() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let eb = find_by_name(&items, "ErrorBoundary");
    assert!(eb.metadata.is_error_boundary);
}

#[test]
fn error_boundary_has_jsx() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let eb = find_by_name(&items, "ErrorBoundary");
    assert!(
        eb.metadata.jsx_elements.contains(&"div".to_string()),
        "jsx: {:?}",
        eb.metadata.jsx_elements
    );
}

#[test]
fn error_boundary_props_type() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let eb = find_by_name(&items, "ErrorBoundary");
    assert_eq!(eb.metadata.props_type.as_deref(), Some("EBProps"));
}

#[test]
fn error_boundary_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let eb = find_by_name(&items, "ErrorBoundary");
    assert!(
        eb.doc_comment.contains("Error boundary"),
        "doc: {:?}",
        eb.doc_comment
    );
}

#[test]
fn pure_counter_is_class_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let pc = find_by_name(&items, "PureCounter");
    assert_eq!(pc.kind, SymbolKind::Component);
    assert!(pc.metadata.is_class_component);
    assert!(!pc.metadata.is_error_boundary);
}

#[test]
fn pure_counter_is_private() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let pc = find_by_name(&items, "PureCounter");
    assert_eq!(pc.visibility, Visibility::Private);
}

#[test]
fn pure_counter_props_type() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let pc = find_by_name(&items, "PureCounter");
    assert_eq!(pc.metadata.props_type.as_deref(), Some("CounterClassProps"));
}
