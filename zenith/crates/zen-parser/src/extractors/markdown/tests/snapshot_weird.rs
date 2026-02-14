use super::common::{Expected, assert_items_match_snapshot, extract_md};
use crate::types::SymbolKind;

#[test]
fn weird_md_snapshot() {
    let src = include_str!("../../../../tests/fixtures/weird.md");
    let items = extract_md(src);

    let expected = vec![
        Expected {
            name: "$",
            signature: "document",
            kind: SymbolKind::Module,
            start_line: 1,
            end_line: 24,
            attrs: &["md:kind:document"],
        },
        Expected {
            name: "Intro",
            signature: "Intro",
            kind: SymbolKind::Module,
            start_line: 1,
            end_line: 2,
            attrs: &["md:kind:heading", "md:level:1"],
        },
        Expected {
            name: "Intro",
            signature: "Intro",
            kind: SymbolKind::Module,
            start_line: 3,
            end_line: 4,
            attrs: &["md:kind:heading", "md:level:2"],
        },
        Expected {
            name: "code-fence-5",
            signature: "```",
            kind: SymbolKind::Property,
            start_line: 5,
            end_line: 8,
            attrs: &["md:kind:code_fence"],
        },
        Expected {
            name: "list-9",
            signature: "list",
            kind: SymbolKind::Property,
            start_line: 9,
            end_line: 13,
            attrs: &["md:kind:list", "md:list_items:3"],
        },
        Expected {
            name: "dup",
            signature: "dup",
            kind: SymbolKind::Property,
            start_line: 13,
            end_line: 14,
            attrs: &["md:kind:link_ref"],
        },
        Expected {
            name: "dup2",
            signature: "dup2",
            kind: SymbolKind::Property,
            start_line: 14,
            end_line: 15,
            attrs: &["md:kind:link_ref"],
        },
        Expected {
            name: "hr-16",
            signature: "---",
            kind: SymbolKind::Property,
            start_line: 16,
            end_line: 17,
            attrs: &["md:kind:thematic_break"],
        },
        Expected {
            name: "Title Alt",
            signature: "Title Alt",
            kind: SymbolKind::Module,
            start_line: 18,
            end_line: 20,
            attrs: &["md:kind:heading", "md:level:2"],
        },
    ];

    assert_items_match_snapshot(&items, &expected);
}
