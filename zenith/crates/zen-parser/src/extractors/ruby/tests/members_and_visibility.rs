use super::*;

#[test]
fn constructor_and_method_owner_metadata_are_set() {
    let items = fixture_items();

    let constructor = find_by_name(&items, "initialize");
    assert_eq!(constructor.kind, SymbolKind::Constructor);
    assert_eq!(
        constructor.metadata.owner_name.as_deref(),
        Some("Billing::Invoice")
    );
    assert_eq!(constructor.metadata.owner_kind, Some(SymbolKind::Class));

    let total = find_by_name(&items, "total");
    assert_eq!(total.kind, SymbolKind::Method);
    assert_eq!(total.visibility, Visibility::Public);
    assert_eq!(
        total.metadata.owner_name.as_deref(),
        Some("Billing::Invoice")
    );
}

#[test]
fn visibility_sections_and_private_class_method_are_respected() {
    let source = r"
class Account
  def initialize
  end

  private
  def token
  end

  protected
  def checksum
  end

  public
  def display
  end

  def self.compute
  end
  private_class_method :compute
end
";
    let items = parse_and_extract(source);

    let token = find_by_name(&items, "token");
    assert_eq!(token.visibility, Visibility::Private);

    let checksum = find_by_name(&items, "checksum");
    assert_eq!(checksum.visibility, Visibility::Protected);

    let display = find_by_name(&items, "display");
    assert_eq!(display.visibility, Visibility::Public);

    let compute = find_by_name(&items, "compute");
    assert_eq!(compute.visibility, Visibility::Private);
    assert!(compute.metadata.is_static_member);
    assert!(compute
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "ruby:private_class_method"));
}
