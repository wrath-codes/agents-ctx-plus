use super::SymbolMetadata;

pub trait CMetadataExt {
    fn push_attribute(&mut self, attribute: impl Into<String>);
    fn push_parameter(&mut self, parameter: impl Into<String>);
}

impl CMetadataExt for SymbolMetadata {
    fn push_attribute(&mut self, attribute: impl Into<String>) {
        self.attributes.push(attribute.into());
    }

    fn push_parameter(&mut self, parameter: impl Into<String>) {
        self.parameters.push(parameter.into());
    }
}
