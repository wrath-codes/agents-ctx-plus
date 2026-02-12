use super::SymbolMetadata;

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
        self.parameters.push(parameter.into());
    }

    fn set_spec(&mut self, spec: Option<String>) {
        self.return_type = spec;
    }

    fn set_guard(&mut self, guard: Option<String>) {
        self.where_clause = guard;
    }

    fn mark_callback_impl(&mut self) {
        self.trait_name = Some("@impl".to_string());
    }

    fn set_delegate_target(&mut self, target: Option<String>) {
        self.for_type = target;
    }

    fn mark_error_type(&mut self) {
        self.is_error_type = true;
    }
}
