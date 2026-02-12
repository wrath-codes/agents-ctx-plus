use super::{SymbolMetadata, common::CommonMetadataExt};

pub trait CMetadataExt {
    fn push_attribute(&mut self, attribute: impl Into<String>);
    fn push_parameter(&mut self, parameter: impl Into<String>);
}

impl CMetadataExt for SymbolMetadata {
    fn push_attribute(&mut self, attribute: impl Into<String>) {
        CommonMetadataExt::push_attribute(self, attribute);
    }

    fn push_parameter(&mut self, parameter: impl Into<String>) {
        CommonMetadataExt::push_parameter(self, parameter);
    }
}
