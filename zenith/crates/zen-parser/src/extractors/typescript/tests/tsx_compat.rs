use super::*;

#[test]
fn tsx_function_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let root = SupportLang::Tsx.ast_grep(source);
    let items = extract(&root, SupportLang::Tsx).expect("extraction should succeed");
    let button = find_by_name(&items, "Button");
    assert_eq!(button.kind, SymbolKind::Function);
    assert_eq!(button.visibility, Visibility::Export);
}

#[test]
fn tsx_default_export_function() {
    let source = include_str!("../../../../tests/fixtures/sample.tsx");
    let root = SupportLang::Tsx.ast_grep(source);
    let items = extract(&root, SupportLang::Tsx).expect("extraction should succeed");
    let app = find_by_name(&items, "App");
    assert_eq!(app.kind, SymbolKind::Function);
    assert!(app.metadata.is_default_export);
}
