use std::borrow::Cow;

use ast_grep_core::language::Language;
use ast_grep_core::matcher::{Pattern, PatternBuilder, PatternError};
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSLanguage};

#[derive(Clone, Copy, Debug)]
pub struct SvelteLang;

impl Language for SvelteLang {
    fn pre_process_pattern<'q>(&self, query: &'q str) -> Cow<'q, str> {
        Cow::Borrowed(query)
    }

    fn kind_to_id(&self, kind: &str) -> u16 {
        self.get_ts_language().id_for_node_kind(kind, true)
    }

    fn field_to_id(&self, field: &str) -> Option<u16> {
        self.get_ts_language()
            .field_id_for_name(field)
            .map(std::num::NonZero::get)
    }

    fn build_pattern(&self, builder: &PatternBuilder) -> Result<Pattern, PatternError> {
        builder.build(|src| StrDoc::try_new(src, *self))
    }
}

impl LanguageExt for SvelteLang {
    fn get_ts_language(&self) -> TSLanguage {
        tree_sitter_svelte_next::LANGUAGE.into()
    }
}
