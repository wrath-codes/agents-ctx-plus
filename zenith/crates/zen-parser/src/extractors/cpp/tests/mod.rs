use ast_grep_language::LanguageExt;

use super::*;
pub(super) use crate::types::{SymbolKind, Visibility};

mod access_specifier_tests;
mod c_11_attribute_tests;
mod class_tests;
mod concept_tests;
mod constexpr_consteval_constinit_tests;
mod decltype_return_type_tests;
mod doc_comment_tests;
mod enum_tests;
mod extern_c_tests;
mod fixture_count_validation_tests;
mod forward_declaration_tests;
mod friend_declaration_tests;
mod function_tests;
mod include_tests;
mod inheritance_tests;
mod inline_namespace_tests;
mod lambda_variable_tests;
mod line_number_tests;
mod method_qualifier_tests;
mod methodbase_tests;
mod minimal_edge_case_tests_for_new_features;
mod misc_edge_cases;
mod mustuseclass_attributed_class_tests;
mod namespace_alias_tests;
mod namespace_tests;
mod nested_types_in_class_tests;
mod preprocessor_define_tests;
mod qualified_identifier_out_of_class_method_tests;
mod requires_clause_tests;
mod smoke_fixture_tests;
mod static_assert_tests;
mod struct_tests;
mod structured_binding_tests;
mod template_alias_tests;
mod template_instantiation_tests;
mod template_tests;
mod templatemethodhost_tests;
mod union_inside_namespace_tests;
mod using_directive_tests;
mod using_typedef_tests;
mod vbase_tests;
mod very_long_namespace_name;
mod wrapper_struct_deduction_guide_tests;

fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
    let root = SupportLang::Cpp.ast_grep(source);
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

#[allow(dead_code)]
fn find_by_name_prefix<'a>(items: &'a [ParsedItem], prefix: &str) -> Vec<&'a ParsedItem> {
    items
        .iter()
        .filter(|i| i.name.starts_with(prefix))
        .collect()
}

fn fixture_items() -> Vec<ParsedItem> {
    let source = include_str!("../../../../tests/fixtures/sample.cpp");
    parse_and_extract(source)
}

// ════════════════════════════════════════════════════════════════
