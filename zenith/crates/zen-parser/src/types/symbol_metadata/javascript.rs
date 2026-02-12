use crate::types::DocSections;

use super::{SymbolMetadata, common::CommonMetadataExt};

pub trait JavaScriptMetadataExt {
    fn mark_async(&mut self);
    fn mark_exported(&mut self);
    fn mark_default_export(&mut self);
    fn mark_generator(&mut self);
    fn set_parameters(&mut self, parameters: Vec<String>);
    fn set_doc_sections(&mut self, doc_sections: DocSections);
    fn set_base_classes(&mut self, base_classes: Vec<String>);
    fn set_methods(&mut self, methods: Vec<String>);
    fn mark_error_type(&mut self);
}

impl JavaScriptMetadataExt for SymbolMetadata {
    fn mark_async(&mut self) {
        CommonMetadataExt::mark_async(self);
    }

    fn mark_exported(&mut self) {
        CommonMetadataExt::mark_exported(self);
    }

    fn mark_default_export(&mut self) {
        CommonMetadataExt::mark_default_export(self);
    }

    fn mark_generator(&mut self) {
        CommonMetadataExt::mark_generator(self);
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

    fn set_methods(&mut self, methods: Vec<String>) {
        CommonMetadataExt::set_methods(self, methods);
    }

    fn mark_error_type(&mut self) {
        CommonMetadataExt::mark_error_type(self);
    }
}
