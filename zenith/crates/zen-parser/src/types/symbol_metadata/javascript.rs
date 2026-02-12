use crate::types::DocSections;

use super::SymbolMetadata;

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
        self.is_async = true;
    }

    fn mark_exported(&mut self) {
        self.is_exported = true;
    }

    fn mark_default_export(&mut self) {
        self.is_default_export = true;
    }

    fn mark_generator(&mut self) {
        self.is_generator = true;
    }

    fn set_parameters(&mut self, parameters: Vec<String>) {
        self.parameters = parameters;
    }

    fn set_doc_sections(&mut self, doc_sections: DocSections) {
        self.doc_sections = doc_sections;
    }

    fn set_base_classes(&mut self, base_classes: Vec<String>) {
        self.base_classes = base_classes;
    }

    fn set_methods(&mut self, methods: Vec<String>) {
        self.methods = methods;
    }

    fn mark_error_type(&mut self) {
        self.is_error_type = true;
    }
}
