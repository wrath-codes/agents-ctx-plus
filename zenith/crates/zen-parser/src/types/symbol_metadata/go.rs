use super::SymbolMetadata;

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
        self.return_type = return_type;
    }

    fn push_parameter(&mut self, parameter: impl Into<String>) {
        self.parameters.push(parameter.into());
    }

    fn set_type_parameters(&mut self, type_parameters: Option<String>) {
        self.type_parameters = type_parameters;
    }

    fn set_receiver(&mut self, receiver: Option<String>) {
        self.for_type = receiver;
    }

    fn set_fields(&mut self, fields: Vec<String>) {
        self.fields = fields;
    }

    fn set_methods(&mut self, methods: Vec<String>) {
        self.methods = methods;
    }

    fn mark_error_type(&mut self) {
        self.is_error_type = true;
    }
}
