use super::*;

#[test]
fn special_keys_use_bracket_path_notation() {
    let source = r#"{
  "a.b": {"x y": 1},
  "x[0]": true,
  "na\"me": "ok",
  "na\u006de": 2
}"#;
    let items = parse_and_extract(source);

    let dotted = find_by_name(&items, "[\"a.b\"]");
    assert_eq!(dotted.kind, SymbolKind::Property);

    let spaced = find_by_name(&items, "[\"a.b\"][\"x y\"]");
    assert_eq!(spaced.metadata.owner_name.as_deref(), Some("[\"a.b\"]"));

    let bracket = find_by_name(&items, "[\"x[0]\"]");
    assert_eq!(bracket.metadata.return_type.as_deref(), Some("boolean"));

    let quoted = items
        .iter()
        .find(|item| item.name.starts_with("[\"na") && item.name.ends_with("\"]"))
        .expect("should find quoted-key path entry");
    assert_eq!(quoted.metadata.return_type.as_deref(), Some("string"));

    let unicode_decoded = find_by_name(&items, "name");
    assert_eq!(
        unicode_decoded.metadata.return_type.as_deref(),
        Some("number")
    );
}
