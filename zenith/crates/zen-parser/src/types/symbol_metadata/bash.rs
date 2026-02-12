use super::SymbolMetadata;

pub trait BashMetadataExt {
    fn push_attribute(&mut self, attribute: impl Into<String>);
    fn push_parameter(&mut self, parameter: impl Into<String>);
}

impl BashMetadataExt for SymbolMetadata {
    fn push_attribute(&mut self, attribute: impl Into<String>) {
        self.attributes.push(attribute.into());
    }

    fn push_parameter(&mut self, parameter: impl Into<String>) {
        self.parameters.push(parameter.into());
    }
}
