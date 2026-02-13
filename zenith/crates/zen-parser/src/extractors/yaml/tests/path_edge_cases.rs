use super::*;

#[test]
fn complex_keys_use_bracket_notation() {
    let source = r#"
"a.b":
  "x y": 1
"x[0]": true
"#;
    let items = parse_and_extract(source);

    let dotted = find_by_name(&items, "[\"a.b\"]");
    assert_eq!(dotted.kind, SymbolKind::Property);

    let spaced = find_by_name(&items, "[\"a.b\"][\"x y\"]");
    assert_eq!(spaced.metadata.owner_name.as_deref(), Some("[\"a.b\"]"));

    let bracket = find_by_name(&items, "[\"x[0]\"]");
    assert_eq!(bracket.metadata.return_type.as_deref(), Some("boolean"));
}
