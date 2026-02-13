use super::*;

#[test]
fn additional_rails_dsl_calls_are_tagged() {
    let source = r"
class Report < ApplicationRecord
  has_one :owner
  has_and_belongs_to_many :tags
  enum :state, { draft: 0, sent: 1 }
  delegate :name, to: :owner
  validate :state_rules
  helper_method :format_state
  around_save :audit
end
";
    let items = parse_and_extract(source);

    let owner = find_by_name(&items, "owner");
    assert!(owner
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:has_one"));

    let tags = find_by_name(&items, "tags");
    assert!(tags
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:habtm"));

    let state = find_by_name(&items, "state");
    assert!(state
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:enum"));

    let delegate = find_by_name(&items, "name");
    assert!(delegate
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:delegate"));

    let validate = find_by_name(&items, "validate");
    assert!(validate
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:validate"));

    let helper_method = find_by_name(&items, "helper_method");
    assert!(helper_method
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:helper_method"));

    let around_save = find_by_name(&items, "around_save");
    assert!(around_save
        .metadata
        .attributes
        .iter()
        .any(|attribute| attribute == "rails:around_save"));
}

#[test]
fn inline_visibility_symbol_directives_are_respected() {
    let source = r"
class Tokenizer
  def digest
  end

  def redacted
  end

  def reveal
  end

  private :digest
  protected :redacted
  public :reveal
end
";
    let items = parse_and_extract(source);

    assert_eq!(
        find_by_name(&items, "digest").visibility,
        Visibility::Private
    );
    assert_eq!(
        find_by_name(&items, "redacted").visibility,
        Visibility::Protected
    );
    assert_eq!(
        find_by_name(&items, "reveal").visibility,
        Visibility::Public
    );
}
