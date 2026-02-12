use super::SymbolMetadata;

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
        self.return_type = return_type;
    }

    fn set_parameters(&mut self, parameters: Vec<String>) {
        self.parameters = parameters;
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

    fn set_generics(&mut self, generics: Option<String>) {
        self.generics = generics;
    }

    fn mark_unsafe(&mut self) {
        self.is_unsafe = true;
    }

    fn mark_error_type(&mut self) {
        self.is_error_type = true;
    }

    fn push_attribute(&mut self, attribute: impl Into<String>) {
        self.attributes.push(attribute.into());
    }
}
