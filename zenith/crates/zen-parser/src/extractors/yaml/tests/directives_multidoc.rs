use super::*;

#[test]
fn directives_and_multidoc_paths_are_preserved() {
    let source = "%YAML 1.2\n---\napp: first\n---\napp: second\n";
    let items = parse_and_extract(source);

    let directive = find_by_name(&items, "doc[0].yaml_directive");
    assert_eq!(directive.kind, SymbolKind::Module);
    assert!(
        directive
            .metadata
            .attributes
            .iter()
            .any(|attr| attr == "yaml:directive:yaml_directive")
    );

    let doc0_app = find_by_name(&items, "doc[0].app");
    assert_eq!(doc0_app.metadata.owner_name.as_deref(), Some("doc[0]"));

    let doc1_app = find_by_name(&items, "doc[1].app");
    assert_eq!(doc1_app.metadata.owner_name.as_deref(), Some("doc[1]"));
}
