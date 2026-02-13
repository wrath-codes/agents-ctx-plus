use ast_grep_core::tree_sitter::LanguageExt;

use crate::types::{ParsedItem, SymbolKind};

#[derive(Debug)]
pub(super) struct Expected<'a> {
    pub(super) name: &'a str,
    pub(super) signature: &'a str,
    pub(super) kind: SymbolKind,
    pub(super) start_line: u32,
    pub(super) end_line: u32,
    pub(super) attrs: &'a [&'a str],
}

pub(super) fn extract_md(src: &str) -> Vec<ParsedItem> {
    let root = crate::parser::MarkdownLang.ast_grep(src);
    crate::extractors::dispatcher::markdown::extract(&root)
        .expect("markdown extraction should succeed")
}

pub(super) fn assert_items_match_snapshot(items: &[ParsedItem], expected: &[Expected<'_>]) {
    assert_eq!(
        items.len(),
        expected.len(),
        "item count mismatch. actual names={:?}",
        items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>()
    );

    for exp in expected {
        let got = items
            .iter()
            .find(|i| {
                i.name == exp.name
                    && i.signature == exp.signature
                    && i.start_line == exp.start_line
                    && i.end_line == exp.end_line
            })
            .unwrap_or_else(|| {
                let available: Vec<_> = items
                    .iter()
                    .map(|i| {
                        format!(
                            "{} / {} [{}-{}]",
                            i.name, i.signature, i.start_line, i.end_line
                        )
                    })
                    .collect();
                panic!(
                    "missing expected item: {} / {} lines {}-{}. available={available:?}",
                    exp.name, exp.signature, exp.start_line, exp.end_line
                )
            });

        assert_eq!(got.kind, exp.kind, "kind mismatch for {}", exp.name);
        for attr in exp.attrs {
            assert!(
                got.metadata.attributes.iter().any(|a| a == attr),
                "missing attr '{}' for {}. attrs={:?}",
                attr,
                exp.name,
                got.metadata.attributes
            );
        }
    }
}

pub(super) fn assert_items_contain_snapshot(
    items: &[ParsedItem],
    expected_subset: &[Expected<'_>],
) {
    for exp in expected_subset {
        let got = items
            .iter()
            .find(|i| {
                i.name == exp.name
                    && i.signature == exp.signature
                    && i.kind == exp.kind
                    && (exp.start_line == 0 || i.start_line == exp.start_line)
                    && (exp.end_line == 0 || i.end_line == exp.end_line)
            })
            .unwrap_or_else(|| {
                let available: Vec<_> = items
                    .iter()
                    .map(|i| {
                        format!(
                            "{} / {} [{}-{}]",
                            i.name, i.signature, i.start_line, i.end_line
                        )
                    })
                    .collect();
                panic!(
                    "missing expected subset item: {} / {} lines {}-{}. available={available:?}",
                    exp.name, exp.signature, exp.start_line, exp.end_line
                )
            });

        for attr in exp.attrs {
            assert!(
                got.metadata.attributes.iter().any(|a| a == attr),
                "missing attr '{}' for {}. attrs={:?}",
                attr,
                exp.name,
                got.metadata.attributes
            );
        }
    }
}
