use crate::types::DocSections;

use super::SymbolMetadata;

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
        self.is_async = true;
    }

    fn mark_exported(&mut self) {
        self.is_exported = true;
    }

    fn mark_default_export(&mut self) {
        self.is_default_export = true;
    }

    fn mark_unsafe(&mut self) {
        self.is_unsafe = true;
    }

    fn mark_error_type(&mut self) {
        self.is_error_type = true;
    }

    fn set_return_type(&mut self, return_type: Option<String>) {
        self.return_type = return_type;
    }

    fn set_type_parameters(&mut self, type_parameters: Option<String>) {
        self.type_parameters = type_parameters;
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

    fn set_implements(&mut self, implements: Vec<String>) {
        self.implements = implements;
    }

    fn set_methods(&mut self, methods: Vec<String>) {
        self.methods = methods;
    }

    fn set_fields(&mut self, fields: Vec<String>) {
        self.fields = fields;
    }

    fn set_variants(&mut self, variants: Vec<String>) {
        self.variants = variants;
    }
}
