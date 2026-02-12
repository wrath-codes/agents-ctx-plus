use crate::types::DocSections;

use super::{SymbolMetadata, common::CommonMetadataExt};

pub trait TypeScriptMetadataExt {
    fn mark_async(&mut self);
    fn mark_exported(&mut self);
    fn mark_default_export(&mut self);
    fn mark_unsafe(&mut self);
    fn mark_error_type(&mut self);
    fn set_return_type(&mut self, return_type: Option<String>);
    fn set_type_parameters(&mut self, type_parameters: Option<String>);
    fn set_parameters(&mut self, parameters: Vec<String>);
    fn set_doc_sections(&mut self, doc_sections: DocSections);
    fn set_base_classes(&mut self, base_classes: Vec<String>);
    fn set_implements(&mut self, implements: Vec<String>);
    fn set_methods(&mut self, methods: Vec<String>);
    fn set_fields(&mut self, fields: Vec<String>);
    fn set_variants(&mut self, variants: Vec<String>);
}

impl TypeScriptMetadataExt for SymbolMetadata {
    fn mark_async(&mut self) {
        CommonMetadataExt::mark_async(self);
    }

    fn mark_exported(&mut self) {
        CommonMetadataExt::mark_exported(self);
    }

    fn mark_default_export(&mut self) {
        CommonMetadataExt::mark_default_export(self);
    }

    fn mark_unsafe(&mut self) {
        CommonMetadataExt::mark_unsafe(self);
    }

    fn mark_error_type(&mut self) {
        CommonMetadataExt::mark_error_type(self);
    }

    fn set_return_type(&mut self, return_type: Option<String>) {
        CommonMetadataExt::set_return_type(self, return_type);
    }

    fn set_type_parameters(&mut self, type_parameters: Option<String>) {
        CommonMetadataExt::set_type_parameters(self, type_parameters);
    }

    fn set_parameters(&mut self, parameters: Vec<String>) {
        CommonMetadataExt::set_parameters(self, parameters);
    }

    fn set_doc_sections(&mut self, doc_sections: DocSections) {
        CommonMetadataExt::set_doc_sections(self, doc_sections);
    }

    fn set_base_classes(&mut self, base_classes: Vec<String>) {
        CommonMetadataExt::set_base_classes(self, base_classes);
    }

    fn set_implements(&mut self, implements: Vec<String>) {
        CommonMetadataExt::set_implements(self, implements);
    }

    fn set_methods(&mut self, methods: Vec<String>) {
        CommonMetadataExt::set_methods(self, methods);
    }

    fn set_fields(&mut self, fields: Vec<String>) {
        CommonMetadataExt::set_fields(self, fields);
    }

    fn set_variants(&mut self, variants: Vec<String>) {
        CommonMetadataExt::set_variants(self, variants);
    }
}
