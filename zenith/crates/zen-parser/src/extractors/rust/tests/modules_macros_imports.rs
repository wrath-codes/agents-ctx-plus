use super::*;

#[test]
fn module_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let internal = find_by_name(&items, "internal");
    assert_eq!(internal.kind, SymbolKind::Module);
}

#[test]
fn macro_definition_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let getter = find_by_name(&items, "make_getter");
    assert_eq!(getter.kind, SymbolKind::Macro);
}

#[test]
fn pub_use_reexport_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let hashmap = find_by_name(&items, "HashMap");
    assert_eq!(hashmap.kind, SymbolKind::Module);
    assert_eq!(hashmap.visibility, Visibility::Public);
    assert!(
        hashmap
            .metadata
            .attributes
            .contains(&"reexport".to_string()),
        "attrs: {:?}",
        hashmap.metadata.attributes
    );
}

#[test]
fn extern_crate_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let alloc = find_by_name(&items, "alloc");
    assert_eq!(alloc.kind, SymbolKind::Module);
    assert!(
        alloc
            .metadata
            .attributes
            .contains(&"extern_crate".to_string()),
        "attrs: {:?}",
        alloc.metadata.attributes
    );
}

#[test]
fn macro_invocation_extracted() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let tl = find_by_name(&items, "thread_local");
    assert_eq!(tl.kind, SymbolKind::Macro);
    assert!(
        tl.metadata
            .attributes
            .contains(&"macro_invocation".to_string()),
        "attrs: {:?}",
        tl.metadata.attributes
    );
}

#[test]
fn macro_export_detected() {
    let source = include_str!("../../../../tests/fixtures/sample.rs");
    let items = parse_and_extract(source);
    let m = find_by_name(&items, "exported_macro");
    assert_eq!(m.kind, SymbolKind::Macro);
    assert!(m.metadata.is_exported, "should be detected as exported");
    assert!(
        m.metadata.attributes.contains(&"macro_export".to_string()),
        "attrs: {:?}",
        m.metadata.attributes
    );
}
