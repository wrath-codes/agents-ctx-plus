use super::*;

#[test]
fn anchors_aliases_and_merge_keys_are_tagged() {
    let source = r"
defaults: &defaults
  retries: 3
service:
  <<: *defaults
  local: &local 9
  mirror: *local
";
    let items = parse_and_extract(source);

    let defaults = find_by_name(&items, "defaults");
    assert!(defaults
        .metadata
        .attributes
        .iter()
        .any(|attr| attr == "yaml:anchor:defaults"));

    let merge = find_by_name(&items, "service[\"<<\"]");
    assert!(merge
        .metadata
        .attributes
        .iter()
        .any(|attr| attr == "yaml:merge_key"));
    assert!(merge
        .metadata
        .attributes
        .iter()
        .any(|attr| attr == "yaml:merge_alias:defaults"));

    let mirror = find_by_name(&items, "service.mirror");
    assert!(mirror
        .metadata
        .attributes
        .iter()
        .any(|attr| attr == "yaml:alias:local"));
    assert!(mirror
        .metadata
        .attributes
        .iter()
        .any(|attr| attr == "yaml:alias_target:service.local"));
}
