use super::*;

#[test]
fn constants_and_fields_are_extracted_from_assignments() {
    let source = r"
class Account
  RATE = 1
  @cache = {}
  @@count = 0
end
";
    let items = parse_and_extract(source);

    let rate = find_by_name(&items, "RATE");
    assert_eq!(rate.kind, SymbolKind::Const);
    assert_eq!(rate.metadata.owner_name.as_deref(), Some("Account"));

    let ivar = find_by_name(&items, "@cache");
    assert_eq!(ivar.kind, SymbolKind::Field);
    assert_eq!(ivar.metadata.owner_name.as_deref(), Some("Account"));

    let cvar = find_by_name(&items, "@@count");
    assert_eq!(cvar.kind, SymbolKind::Field);
    assert_eq!(cvar.metadata.owner_name.as_deref(), Some("Account"));
}

#[test]
fn top_level_def_is_function() {
    let source = r"
def normalize_token(value)
  value.to_s.strip
end
";
    let items = parse_and_extract(source);

    let function = find_by_name(&items, "normalize_token");
    assert_eq!(function.kind, SymbolKind::Function);
    assert!(function.metadata.owner_name.is_none());
}
