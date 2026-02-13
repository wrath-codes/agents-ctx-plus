use super::*;

#[test]
fn anonymous_class_members_have_anonymous_owner() {
    let source = r"
<?php
$tmp = new class() {
    public string $name;
    public function value(): string { return $this->name; }
};
";

    let items = parse_and_extract(source);
    let method = items
        .iter()
        .find(|i| i.kind == SymbolKind::Method && i.name == "value")
        .expect("expected anonymous class method");
    assert!(method
        .metadata
        .owner_name
        .as_deref()
        .is_some_and(|o| o.starts_with("<anonymous_class@")));

    let property = items
        .iter()
        .find(|i| i.kind == SymbolKind::Property && i.name == "name")
        .expect("expected anonymous class property");
    assert!(property
        .metadata
        .owner_name
        .as_deref()
        .is_some_and(|o| o.starts_with("<anonymous_class@")));
}

#[test]
fn globals_and_statics_are_owned_by_enclosing_method() {
    let source = r"
<?php
class Box {
  public function run(): void {
    global $counter;
    static $memo = [];
  }
}
";

    let items = parse_and_extract(source);
    let counter = find_by_name(&items, "counter");
    assert_eq!(counter.kind, SymbolKind::Static);
    assert_eq!(counter.metadata.owner_name.as_deref(), Some("run"));

    let memo = find_by_name(&items, "memo");
    assert_eq!(memo.kind, SymbolKind::Static);
    assert_eq!(memo.metadata.owner_name.as_deref(), Some("run"));
}

#[test]
fn promoted_properties_remain_field_kind() {
    let source = r"
<?php
class User {
  public function __construct(private int $id) {}
}
";

    let items = parse_and_extract(source);
    let id = find_by_name(&items, "id");
    assert_eq!(id.kind, SymbolKind::Field);
}
