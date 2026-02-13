use super::*;

#[test]
fn fixture_parses_and_has_root() {
    let items = fixture_items();
    assert!(!items.is_empty());
    let root = find_by_name(&items, "$");
    assert_eq!(root.kind, SymbolKind::Module);
    assert!(has_attr(root, "rst:kind:document"));
}

#[test]
fn sections_and_owners_are_extracted() {
    let items = fixture_items();
    let title = find_by_name(&items, "Sample RST");
    assert_eq!(title.kind, SymbolKind::Module);
    assert!(has_attr(title, "rst:kind:section"));

    let child = find_by_name(&items, "Usage");
    assert_eq!(child.metadata.owner_name.as_deref(), Some("Sample RST"));
    assert!(has_attr(child, "rst:path:Sample RST/Usage"));
}

#[test]
fn directives_targets_and_footnotes_are_extracted() {
    let items = fixture_items();

    let directive = find_by_name(&items, "directive:note");
    assert_eq!(directive.kind, SymbolKind::Property);
    assert!(has_attr(directive, "rst:kind:directive"));
    assert!(has_attr(directive, "rst:directive:note"));
    assert!(has_attr(directive, "rst:directive_options:1"));

    let target = find_by_name(&items, "target:target-name");
    assert_eq!(target.kind, SymbolKind::Property);
    assert!(has_attr(target, "rst:kind:target"));

    let foot = find_by_name(&items, "footnote:#");
    assert_eq!(foot.kind, SymbolKind::Property);
    assert!(has_attr(foot, "rst:kind:footnote"));

    let code = find_by_name(&items, "directive:code-block");
    assert!(has_attr(code, "rst:code_directive"));
    assert!(has_attr(code, "rst:code_lang:python"));

    let include = find_by_name(&items, "directive:include");
    assert!(has_attr(include, "rst:include_directive"));
    assert!(has_attr(include, "rst:include:path:included.rst"));
}
