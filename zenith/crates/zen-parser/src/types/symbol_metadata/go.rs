use super::{SymbolMetadata, common::CommonMetadataExt};

pub trait GoMetadataExt {
    fn set_return_type(&mut self, return_type: Option<String>);
    fn push_parameter(&mut self, parameter: impl Into<String>);
    fn set_type_parameters(&mut self, type_parameters: Option<String>);
    fn set_receiver(&mut self, receiver: Option<String>);
    fn set_fields(&mut self, fields: Vec<String>);
    fn set_methods(&mut self, methods: Vec<String>);
    fn mark_error_type(&mut self);
}

impl GoMetadataExt for SymbolMetadata {
    fn set_return_type(&mut self, return_type: Option<String>) {
        CommonMetadataExt::set_return_type(self, return_type);
    }

    fn push_parameter(&mut self, parameter: impl Into<String>) {
        CommonMetadataExt::push_parameter(self, parameter);
    }

    fn set_type_parameters(&mut self, type_parameters: Option<String>) {
        CommonMetadataExt::set_type_parameters(self, type_parameters);
    }

    fn set_receiver(&mut self, receiver: Option<String>) {
        CommonMetadataExt::set_for_type(self, receiver);
    }

    fn set_fields(&mut self, fields: Vec<String>) {
        CommonMetadataExt::set_fields(self, fields);
    }

    fn set_methods(&mut self, methods: Vec<String>) {
        CommonMetadataExt::set_methods(self, methods);
    }

    fn mark_error_type(&mut self) {
        CommonMetadataExt::mark_error_type(self);
    }
}
