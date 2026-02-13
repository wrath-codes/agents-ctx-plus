use super::*;

fn has_inline(items: &[ParsedItem], kind: &str) -> bool {
    items.iter().any(|i| {
        i.metadata
            .attributes
            .iter()
            .any(|a| a == &format!("rst:kind:{kind}"))
    })
}

#[test]
fn list_and_block_items_are_extracted() {
    let items = fixture_items();

    let bullet = items
        .iter()
        .find(|i| has_attr(i, "rst:kind:bullet_list"))
        .expect("bullet list should exist");
    assert_eq!(bullet.kind, SymbolKind::Property);

    let enumerated = items
        .iter()
        .find(|i| has_attr(i, "rst:kind:enumerated_list"))
        .expect("enumerated list should exist");
    assert_eq!(enumerated.kind, SymbolKind::Property);

    let code_directive = items
        .iter()
        .find(|i| has_attr(i, "rst:code_directive"))
        .expect("code directive should exist");
    assert_eq!(code_directive.kind, SymbolKind::Property);
}

#[test]
fn inline_markup_is_extracted() {
    let items = fixture_items();
    assert!(has_inline(&items, "reference"));
    assert!(has_inline(&items, "standalone_hyperlink"));
    assert!(has_inline(&items, "literal"));
    assert!(has_inline(&items, "strong"));
    assert!(has_inline(&items, "emphasis"));
    assert!(has_inline(&items, "interpreted_text"));
    assert!(has_inline(&items, "substitution_reference"));
}

#[test]
fn resolves_targets_and_marks_broken_references() {
    let items = fixture_items();

    let resolved = items
        .iter()
        .find(|i| {
            has_attr(i, "rst:kind:reference")
                && i.metadata
                    .attributes
                    .iter()
                    .any(|a| a.starts_with("rst:ref_target:target:target-name"))
        })
        .expect("resolved reference should exist");
    assert!(has_attr(resolved, "rst:ref_label:target-name"));

    let broken = items
        .iter()
        .find(|i| {
            has_attr(i, "rst:kind:reference")
                && i.metadata
                    .attributes
                    .iter()
                    .any(|a| a.starts_with("rst:broken_reference:"))
        })
        .expect("broken reference should exist");
    assert!(broken
        .metadata
        .attributes
        .iter()
        .any(|a| a == "rst:broken_reference:missing-ref"));
}

#[test]
fn extracts_substitutions_and_table_metadata() {
    let items = fixture_items();

    let subst = find_by_name(&items, "substitution:brand");
    assert!(has_attr(subst, "rst:kind:substitution_definition"));

    let table = items
        .iter()
        .find(|i| has_attr(i, "rst:kind:grid_table"))
        .expect("grid table item should exist");
    assert!(table
        .metadata
        .attributes
        .iter()
        .any(|a| a.starts_with("rst:table_rows:")));
    assert!(table
        .metadata
        .attributes
        .iter()
        .any(|a| a.starts_with("rst:table_cols:")));
}
