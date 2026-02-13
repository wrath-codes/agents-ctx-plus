use super::*;

#[test]
fn concern_included_and_class_methods_are_tagged() {
    let source = r"
module Auditable
  extend ActiveSupport::Concern

  included do
    has_many :events
    before_save :stamp
  end

  class_methods do
    def recent
    end
  end
end
";
    let items = parse_and_extract(source);

    let events = find_by_name(&items, "events");
    assert_eq!(events.kind, SymbolKind::Property);
    assert!(events
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:concern:included"));

    let recent = find_by_name(&items, "recent");
    assert_eq!(recent.kind, SymbolKind::Method);
    assert!(recent.metadata.is_static_member);
    assert!(recent
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:concern:class_methods"));
}
