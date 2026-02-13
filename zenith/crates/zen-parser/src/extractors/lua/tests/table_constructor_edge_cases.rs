use super::*;

#[test]
fn table_constructor_emits_fields_and_methods() {
    let source = r#"
local Config = {
  build = function(x) return x end,
  mode = "fast",
  ["timeout"] = 10,
  ["init"] = function() return true end,
}
"#;

    let items = parse_and_extract(source);

    let build = find_by_name(&items, "build");
    assert_eq!(build.kind, SymbolKind::Method);
    assert_eq!(build.metadata.owner_name.as_deref(), Some("Config"));
    assert!(
        build
            .metadata
            .attributes
            .iter()
            .any(|a| a == "callable_origin:table_ctor")
    );

    let mode = find_by_name(&items, "mode");
    assert_eq!(mode.kind, SymbolKind::Field);
    assert_eq!(mode.metadata.owner_name.as_deref(), Some("Config"));

    let timeout = find_by_name(&items, "timeout");
    assert_eq!(timeout.kind, SymbolKind::Field);
    assert_eq!(timeout.metadata.owner_name.as_deref(), Some("Config"));

    let init = find_by_name(&items, "init");
    assert_eq!(init.kind, SymbolKind::Method);
    assert_eq!(init.metadata.owner_name.as_deref(), Some("Config"));
}
