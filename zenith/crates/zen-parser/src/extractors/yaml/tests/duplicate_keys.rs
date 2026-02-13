use super::*;

#[test]
fn duplicate_keys_are_tagged() {
    let items = parse_and_extract("a: 1\na: 2\n");
    let matches = find_all_by_name(&items, "a");
    assert!(matches.iter().any(|item| {
        item.metadata
            .attributes
            .iter()
            .any(|attr| attr == "yaml:duplicate_key:a")
    }));
}
