use super::common::{Expected, assert_items_contain_snapshot, extract_md};
use crate::types::SymbolKind;

#[test]
fn malformed_md_resilience_subset_snapshot() {
    let src = include_str!("../../../../tests/fixtures/malformed.md");
    let items = extract_md(src);

    assert!(!items.is_empty());

    let expected_subset = vec![
        Expected {
            name: "$",
            signature: "document",
            kind: SymbolKind::Module,
            start_line: 0,
            end_line: 0,
            attrs: &["md:kind:document"],
        },
        Expected {
            name: "Broken",
            signature: "Broken",
            kind: SymbolKind::Module,
            start_line: 1,
            end_line: 2,
            attrs: &["md:kind:heading", "md:level:1"],
        },
        Expected {
            name: "list-5",
            signature: "list",
            kind: SymbolKind::Property,
            start_line: 5,
            end_line: 8,
            attrs: &["md:kind:list", "md:list_items:2"],
        },
        Expected {
            name: "ok",
            signature: "ok",
            kind: SymbolKind::Property,
            start_line: 8,
            end_line: 9,
            attrs: &["md:kind:link_ref"],
        },
        Expected {
            name: "hr-11",
            signature: "---",
            kind: SymbolKind::Property,
            start_line: 11,
            end_line: 12,
            attrs: &["md:kind:thematic_break"],
        },
    ];

    assert_items_contain_snapshot(&items, &expected_subset);
}
