use super::{SymbolMetadata, common::CommonMetadataExt};

pub trait CppMetadataExt {
    fn set_return_type(&mut self, return_type: Option<String>);
    fn set_parameters(&mut self, parameters: Vec<String>);
    fn set_fields(&mut self, fields: Vec<String>);
    fn set_methods(&mut self, methods: Vec<String>);
    fn set_variants(&mut self, variants: Vec<String>);
    fn set_base_classes(&mut self, base_classes: Vec<String>);
    fn set_generics(&mut self, generics: Option<String>);
    fn mark_unsafe(&mut self);
    fn mark_error_type(&mut self);
    fn push_attribute(&mut self, attribute: impl Into<String>);
}

impl CppMetadataExt for SymbolMetadata {
    fn set_return_type(&mut self, return_type: Option<String>) {
        CommonMetadataExt::set_return_type(self, return_type);
    }

    fn set_parameters(&mut self, parameters: Vec<String>) {
        CommonMetadataExt::set_parameters(self, parameters);
    }

    fn set_fields(&mut self, fields: Vec<String>) {
        CommonMetadataExt::set_fields(self, fields);
    }

    fn set_methods(&mut self, methods: Vec<String>) {
        CommonMetadataExt::set_methods(self, methods);
    }

    fn set_variants(&mut self, variants: Vec<String>) {
        CommonMetadataExt::set_variants(self, variants);
    }

    fn set_base_classes(&mut self, base_classes: Vec<String>) {
        CommonMetadataExt::set_base_classes(self, base_classes);
    }

    fn set_generics(&mut self, generics: Option<String>) {
        CommonMetadataExt::set_generics(self, generics);
    }

    fn mark_unsafe(&mut self) {
        CommonMetadataExt::mark_unsafe(self);
    }

    fn mark_error_type(&mut self) {
        CommonMetadataExt::mark_error_type(self);
    }

    fn push_attribute(&mut self, attribute: impl Into<String>) {
        CommonMetadataExt::push_attribute(self, attribute);
    }
}
