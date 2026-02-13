use super::*;

#[test]
fn extracts_document_and_embedded_blocks() {
    let items = fixture_items();

    let root = find_by_name(&items, "$");
    assert_eq!(root.kind, SymbolKind::Module);
    assert!(has_attr(root, "svelte:kind:document"));

    let module_script = find_by_name(&items, "script:module");
    assert!(has_attr(module_script, "svelte:kind:script"));
    assert!(has_attr(module_script, "svelte:script_lang:ts"));
    assert!(has_attr(module_script, "svelte:embedded_parser:typescript"));

    let instance_script = find_by_name(&items, "script:instance");
    assert!(has_attr(instance_script, "svelte:uses_props_api"));

    let style = find_by_name(&items, "style");
    assert!(has_attr(style, "svelte:kind:style"));
    assert!(has_attr(style, "svelte:style_lang:css"));
    assert!(has_attr(style, "svelte:embedded_parser:css"));
    assert!(has_attr(style, "svelte:style_global_selector"));
}

#[test]
fn extracts_elements_and_components() {
    let items = fixture_items();

    let main = find_by_name(&items, "main");
    assert_eq!(main.kind, SymbolKind::Struct);
    assert!(has_attr(main, "svelte:kind:element"));
    assert!(has_attr(main, "svelte:embedded_parser:html"));

    let component = find_by_name(&items, "MyCard");
    assert_eq!(component.kind, SymbolKind::Component);
    assert!(has_attr(component, "svelte:component"));

    let duplicate = items
        .iter()
        .find(|i| i.name.starts_with("duplicate-id:main:"))
        .expect("duplicate id should be detected");
    assert!(has_attr(duplicate, "svelte:kind:duplicate_id"));
}
