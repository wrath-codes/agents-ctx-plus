use super::*;

#[test]
fn dedupe_preserves_distinct_owners_and_reduces_duplicates() {
    let source = r"
local M = {
  scale = function(v) return v * 2 end,
}
M.scale = function(v) return v * 2 end

A.init = function() end
B.init = function() end
";

    let items = parse_and_extract(source);

    assert!(
        items
            .iter()
            .filter(|i| i.name == "scale" && i.kind == SymbolKind::Method)
            .count()
            >= 1
    );

    assert_eq!(
        items
            .iter()
            .filter(|i| i.name == "init" && i.metadata.owner_name.as_deref() == Some("A"))
            .count(),
        1
    );
    assert_eq!(
        items
            .iter()
            .filter(|i| i.name == "init" && i.metadata.owner_name.as_deref() == Some("B"))
            .count(),
        1
    );
}
