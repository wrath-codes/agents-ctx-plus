use super::{SymbolMetadata, common::CommonMetadataExt};

pub trait BashMetadataExt {
    fn push_attribute(&mut self, attribute: impl Into<String>);
    fn push_parameter(&mut self, parameter: impl Into<String>);
}

impl BashMetadataExt for SymbolMetadata {
    fn push_attribute(&mut self, attribute: impl Into<String>) {
        CommonMetadataExt::push_attribute(self, attribute);
    }

    fn push_parameter(&mut self, parameter: impl Into<String>) {
        CommonMetadataExt::push_parameter(self, parameter);
    }
}
