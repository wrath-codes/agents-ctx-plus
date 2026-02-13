use super::*;

#[test]
fn pointer_and_value_receivers_have_owner_metadata() {
    let source = r#"package demo
type Config struct{}
func (c *Config) Run() error { return nil }
func (c Config) Name() string { return "" }
"#;

    let items = parse_and_extract(source);

    let run = find_by_name(&items, "Run");
    assert_eq!(run.kind, SymbolKind::Method);
    assert_eq!(run.metadata.owner_name.as_deref(), Some("Config"));
    assert_eq!(run.metadata.owner_kind, Some(SymbolKind::Struct));
    assert!(!run.metadata.is_static_member);
    assert!(
        run.metadata
            .attributes
            .iter()
            .any(|a| a == "receiver:pointer")
    );

    let name = find_by_name(&items, "Name");
    assert_eq!(name.metadata.owner_name.as_deref(), Some("Config"));
    assert!(
        name.metadata
            .attributes
            .iter()
            .any(|a| a == "receiver:value")
    );
}
