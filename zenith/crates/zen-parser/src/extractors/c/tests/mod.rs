use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod additional_coverage_multi_dimensional_arrays;
mod additional_coverage_signature_quality;
mod additional_coverage_typedef_union;
mod array_declaration_tests;
mod doc_comment_style_tests;
mod enum_tests;
mod fixture_parsing;
mod function_pointer_variable_tests;
mod function_tests;
mod gap_1_extended_nested_preprocif_preprocelif_preprocelse;
mod gap_1_preprocif_preprocelif_preprocelse;
mod gap_2_extended_multi_variable_edge_cases;
mod gap_2_multi_variable_declarations;
mod gap_3_extended_volatile_combinations;
mod gap_3_volatile_qualifier;
mod gap_4_extended_register_edge_cases;
mod gap_4_register_storage_class;
mod gap_5_attribute;
mod gap_5_extended_attribute_variations;
mod gap_6_c11_qualifiers_noreturn_atomic;
mod gap_6_extended_c11_qualifier_variations;
mod gap_7_anonymous_struct_union_in_fields;
mod gap_7_extended_anonymous_aggregates;
mod inline_edge_case_tests;
mod line_number_tests;
mod pointer_to_pointer;
mod preprocessor_tests;
mod prototype_tests;
mod static_assert_tests;
mod struct_tests;
mod typedef_tests;
mod union_tests;
mod updated_fixture_count;
mod variable_tests;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::C.ast_grep(source);
    extract(&root, source).expect("extraction should succeed")
}

fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
    items.iter().find(|i| i.name == name).unwrap_or_else(|| {
        let available: Vec<_> = items
            .iter()
            .map(|i| format!("{:?}: {}", i.kind, &i.name))
            .collect();
        panic!(
            "item {name:?} not found. Available items:\n{}",
            available.join("\n")
        );
    })
}

fn find_all_by_kind(items: &[ParsedItem], kind: SymbolKind) -> Vec<&ParsedItem> {
    items.iter().filter(|i| i.kind == kind).collect()
}

fn find_by_name_prefix<'a>(items: &'a [ParsedItem], prefix: &str) -> Vec<&'a ParsedItem> {
    items
        .iter()
        .filter(|i| i.name.starts_with(prefix))
        .collect()
}
