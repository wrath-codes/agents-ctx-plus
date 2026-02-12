use super::{SymbolMetadata, common::CommonMetadataExt};

pub trait ElixirMetadataExt {
    fn push_parameter(&mut self, parameter: impl Into<String>);
    fn set_spec(&mut self, spec: Option<String>);
    fn set_guard(&mut self, guard: Option<String>);
    fn mark_callback_impl(&mut self);
    fn set_delegate_target(&mut self, target: Option<String>);
    fn mark_error_type(&mut self);
}

impl ElixirMetadataExt for SymbolMetadata {
    fn push_parameter(&mut self, parameter: impl Into<String>) {
        CommonMetadataExt::push_parameter(self, parameter);
    }

    fn set_spec(&mut self, spec: Option<String>) {
        CommonMetadataExt::set_return_type(self, spec);
    }

    fn set_guard(&mut self, guard: Option<String>) {
        CommonMetadataExt::set_where_clause(self, guard);
    }

    fn mark_callback_impl(&mut self) {
        CommonMetadataExt::set_trait_name(self, Some("@impl".to_string()));
    }

    fn set_delegate_target(&mut self, target: Option<String>) {
        CommonMetadataExt::set_for_type(self, target);
    }

    fn mark_error_type(&mut self) {
        CommonMetadataExt::mark_error_type(self);
    }
}
