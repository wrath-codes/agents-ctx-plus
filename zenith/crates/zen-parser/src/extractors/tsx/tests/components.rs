use super::*;

// ── Component detection ────────────────────────────────────────

#[test]
fn button_is_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let btn = find_by_name(&items, "Button");
    assert_eq!(btn.kind, SymbolKind::Component);
    assert!(btn.metadata.is_component);
}

#[test]
fn button_exported() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let btn = find_by_name(&items, "Button");
    assert_eq!(btn.visibility, Visibility::Export);
    assert!(btn.metadata.is_exported);
}

#[test]
fn button_has_props_type() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let btn = find_by_name(&items, "Button");
    assert_eq!(btn.metadata.props_type.as_deref(), Some("ButtonProps"));
}

#[test]
fn button_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let btn = find_by_name(&items, "Button");
    assert!(
        btn.doc_comment.contains("Primary button component"),
        "doc: {:?}",
        btn.doc_comment
    );
}

#[test]
fn button_jsx_elements() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let btn = find_by_name(&items, "Button");
    assert!(
        btn.metadata.jsx_elements.contains(&"button".to_string()),
        "jsx: {:?}",
        btn.metadata.jsx_elements
    );
}

// ── Private (non-exported) component ───────────────────────────

#[test]
fn sidebar_is_private_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let sb = find_by_name(&items, "Sidebar");
    assert_eq!(sb.kind, SymbolKind::Component);
    assert!(sb.metadata.is_component);
    assert_eq!(sb.visibility, Visibility::Private);
}

// ── Arrow component (React.FC) ─────────────────────────────────

#[test]
fn usercard_is_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let uc = find_by_name(&items, "UserCard");
    assert_eq!(uc.kind, SymbolKind::Component);
    assert!(uc.metadata.is_component);
}

#[test]
fn usercard_props_type() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let uc = find_by_name(&items, "UserCard");
    assert_eq!(uc.metadata.props_type.as_deref(), Some("UserCardProps"));
}

#[test]
fn usercard_has_hooks() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let uc = find_by_name(&items, "UserCard");
    assert!(
        uc.metadata.hooks_used.contains(&"useCallback".to_string()),
        "hooks: {:?}",
        uc.metadata.hooks_used
    );
}

#[test]
fn usercard_jsx_elements() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let uc = find_by_name(&items, "UserCard");
    assert!(
        uc.metadata.jsx_elements.contains(&"div".to_string()),
        "jsx: {:?}",
        uc.metadata.jsx_elements
    );
}

// ── Generic component ──────────────────────────────────────────

#[test]
fn list_is_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let list = find_by_name(&items, "List");
    assert_eq!(list.kind, SymbolKind::Component);
    assert!(list.metadata.is_component);
}

#[test]
fn list_has_type_params() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let list = find_by_name(&items, "List");
    assert!(
        list.metadata
            .type_parameters
            .as_deref()
            .is_some_and(|t| t.contains('T')),
        "type_params: {:?}",
        list.metadata.type_parameters
    );
}

#[test]
fn list_props_type() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let list = find_by_name(&items, "List");
    assert!(
        list.metadata
            .props_type
            .as_deref()
            .is_some_and(|p| p.contains("ListProps")),
        "props: {:?}",
        list.metadata.props_type
    );
}

// ── Counter (multiple hooks) ───────────────────────────────────

#[test]
fn counter_is_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Counter");
    assert_eq!(c.kind, SymbolKind::Component);
    assert!(c.metadata.is_component);
}

#[test]
fn counter_hooks_used() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Counter");
    for hook in &["useState", "useEffect", "useRef", "useMemo"] {
        assert!(
            c.metadata.hooks_used.contains(&(*hook).to_string()),
            "missing hook {hook}: {:?}",
            c.metadata.hooks_used
        );
    }
}

#[test]
fn counter_jsx_elements() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let c = find_by_name(&items, "Counter");
    assert!(
        c.metadata.jsx_elements.contains(&"div".to_string()),
        "jsx: {:?}",
        c.metadata.jsx_elements
    );
    assert!(
        c.metadata.jsx_elements.contains(&"Button".to_string()),
        "jsx: {:?}",
        c.metadata.jsx_elements
    );
}

// ── TodoApp (useReducer) ───────────────────────────────────────

#[test]
fn todo_app_is_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "TodoApp");
    assert_eq!(t.kind, SymbolKind::Component);
    assert!(t.metadata.is_component);
}

#[test]
fn todo_app_uses_reducer() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let t = find_by_name(&items, "TodoApp");
    assert!(
        t.metadata.hooks_used.contains(&"useReducer".to_string()),
        "hooks: {:?}",
        t.metadata.hooks_used
    );
}

// ── Default export component ───────────────────────────────────

#[test]
fn app_is_default_export_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let app = find_by_name(&items, "App");
    assert_eq!(app.kind, SymbolKind::Component);
    assert!(app.metadata.is_default_export);
    assert!(app.metadata.is_component);
}

#[test]
fn app_hooks_used() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let app = find_by_name(&items, "App");
    assert!(
        app.metadata.hooks_used.contains(&"useState".to_string()),
        "hooks: {:?}",
        app.metadata.hooks_used
    );
    assert!(
        app.metadata.hooks_used.contains(&"useCallback".to_string()),
        "hooks: {:?}",
        app.metadata.hooks_used
    );
}

#[test]
fn app_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let app = find_by_name(&items, "App");
    assert!(
        app.doc_comment.contains("Main application shell"),
        "doc: {:?}",
        app.doc_comment
    );
}

#[test]
fn app_renders_sidebar() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let app = find_by_name(&items, "App");
    assert!(
        app.metadata.jsx_elements.contains(&"Sidebar".to_string()),
        "jsx: {:?}",
        app.metadata.jsx_elements
    );
}

// ── Suspense boundary ──────────────────────────────────────────

#[test]
fn page_with_suspense_is_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "PageWithSuspense");
    assert_eq!(p.kind, SymbolKind::Component);
    assert!(p.metadata.is_component);
}

#[test]
fn page_with_suspense_renders_suspense() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "PageWithSuspense");
    assert!(
        p.metadata.jsx_elements.contains(&"Suspense".to_string()),
        "jsx: {:?}",
        p.metadata.jsx_elements
    );
}

#[test]
fn page_with_suspense_renders_lazy_component() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "PageWithSuspense");
    assert!(
        p.metadata
            .jsx_elements
            .contains(&"LazySettings".to_string()),
        "jsx: {:?}",
        p.metadata.jsx_elements
    );
}

#[test]
fn page_with_suspense_has_jsdoc() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let items = parse_and_extract(source);
    let p = find_by_name(&items, "PageWithSuspense");
    assert!(
        p.doc_comment.contains("suspense boundary"),
        "doc: {:?}",
        p.doc_comment
    );
}
