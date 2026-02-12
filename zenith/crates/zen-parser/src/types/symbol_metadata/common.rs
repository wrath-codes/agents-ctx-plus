use crate::types::{DocSections, SymbolKind};

use super::SymbolMetadata;

pub trait CommonMetadataExt {
    fn mark_async(&mut self);
    fn mark_unsafe(&mut self);
    fn mark_exported(&mut self);
    fn mark_default_export(&mut self);
    fn mark_error_type(&mut self);
    fn mark_generator(&mut self);

    fn set_return_type(&mut self, return_type: Option<String>);
    fn set_generics(&mut self, generics: Option<String>);
    fn set_type_parameters(&mut self, type_parameters: Option<String>);
    fn set_parameters(&mut self, parameters: Vec<String>);
    fn set_owner_name(&mut self, owner_name: Option<String>);
    fn set_owner_kind(&mut self, owner_kind: Option<SymbolKind>);
    fn mark_static_member(&mut self);
    fn set_doc_sections(&mut self, doc_sections: DocSections);
    fn set_where_clause(&mut self, where_clause: Option<String>);
    fn set_trait_name(&mut self, trait_name: Option<String>);
    fn set_for_type(&mut self, for_type: Option<String>);
    fn set_implements(&mut self, implements: Vec<String>);

    fn set_fields(&mut self, fields: Vec<String>);
    fn set_methods(&mut self, methods: Vec<String>);
    fn set_variants(&mut self, variants: Vec<String>);
    fn set_base_classes(&mut self, base_classes: Vec<String>);

    fn push_attribute(&mut self, attribute: impl Into<String>);
    fn push_parameter(&mut self, parameter: impl Into<String>);
}

impl CommonMetadataExt for SymbolMetadata {
    fn mark_async(&mut self) {
        self.is_async = true;
    }

    fn mark_unsafe(&mut self) {
        self.is_unsafe = true;
    }

    fn mark_exported(&mut self) {
        self.is_exported = true;
    }

    fn mark_default_export(&mut self) {
        self.is_default_export = true;
    }

    fn mark_error_type(&mut self) {
        self.is_error_type = true;
    }

    fn mark_generator(&mut self) {
        self.is_generator = true;
    }

    fn set_return_type(&mut self, return_type: Option<String>) {
        self.return_type = return_type;
    }

    fn set_generics(&mut self, generics: Option<String>) {
        self.generics = generics;
    }

    fn set_type_parameters(&mut self, type_parameters: Option<String>) {
        self.type_parameters = type_parameters;
    }

    fn set_parameters(&mut self, parameters: Vec<String>) {
        self.parameters = parameters;
    }

    fn set_owner_name(&mut self, owner_name: Option<String>) {
        self.owner_name = owner_name;
    }

    fn set_owner_kind(&mut self, owner_kind: Option<SymbolKind>) {
        self.owner_kind = owner_kind;
    }

    fn mark_static_member(&mut self) {
        self.is_static_member = true;
    }

    fn set_doc_sections(&mut self, doc_sections: DocSections) {
        self.doc_sections = doc_sections;
    }

    fn set_where_clause(&mut self, where_clause: Option<String>) {
        self.where_clause = where_clause;
    }

    fn set_trait_name(&mut self, trait_name: Option<String>) {
        self.trait_name = trait_name;
    }

    fn set_for_type(&mut self, for_type: Option<String>) {
        self.for_type = for_type;
    }

    fn set_implements(&mut self, implements: Vec<String>) {
        self.implements = implements;
    }

    fn set_fields(&mut self, fields: Vec<String>) {
        self.fields = fields;
    }

    fn set_methods(&mut self, methods: Vec<String>) {
        self.methods = methods;
    }

    fn set_variants(&mut self, variants: Vec<String>) {
        self.variants = variants;
    }

    fn set_base_classes(&mut self, base_classes: Vec<String>) {
        self.base_classes = base_classes;
    }

    fn push_attribute(&mut self, attribute: impl Into<String>) {
        self.attributes.push(attribute.into());
    }

    fn push_parameter(&mut self, parameter: impl Into<String>) {
        self.parameters.push(parameter.into());
    }
}
