use super::common::{Expected, assert_items_match_snapshot, extract_md};
use crate::types::SymbolKind;

#[test]
fn sample_md_snapshot() {
    let src = include_str!("../../../../tests/fixtures/sample.md");
    let items = extract_md(src);

    let expected = vec![
        Expected {
            name: "$",
            signature: "document",
            kind: SymbolKind::Module,
            start_line: 1,
            end_line: 27,
            attrs: &["md:kind:document"],
        },
        Expected {
            name: "Intro",
            signature: "Intro",
            kind: SymbolKind::Module,
            start_line: 5,
            end_line: 6,
            attrs: &["md:kind:heading", "md:level:1"],
        },
        Expected {
            name: "Install",
            signature: "Install",
            kind: SymbolKind::Module,
            start_line: 9,
            end_line: 10,
            attrs: &["md:kind:heading", "md:level:2"],
        },
        Expected {
            name: "code-fence-11",
            signature: "```bash",
            kind: SymbolKind::Property,
            start_line: 11,
            end_line: 14,
            attrs: &["md:kind:code_fence", "md:code_lang:bash"],
        },
        Expected {
            name: "list-15",
            signature: "list",
            kind: SymbolKind::Property,
            start_line: 15,
            end_line: 18,
            attrs: &["md:kind:list", "md:list_items:2"],
        },
        Expected {
            name: "table-18",
            signature: "table",
            kind: SymbolKind::Property,
            start_line: 18,
            end_line: 21,
            attrs: &["md:kind:table", "md:table_rows:3"],
        },
        Expected {
            name: "ref",
            signature: "ref",
            kind: SymbolKind::Property,
            start_line: 22,
            end_line: 23,
            attrs: &["md:kind:link_ref"],
        },
        Expected {
            name: "frontmatter-yaml-1",
            signature: "frontmatter:yaml",
            kind: SymbolKind::Property,
            start_line: 1,
            end_line: 4,
            attrs: &["md:kind:frontmatter:yaml"],
        },
    ];

    assert_items_match_snapshot(&items, &expected);
}
