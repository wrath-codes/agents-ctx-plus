use super::{SymbolMetadata, common::CommonMetadataExt};

pub trait PythonMetadataExt {
    fn mark_exported(&mut self);
    fn mark_dataclass(&mut self);
    fn mark_pydantic(&mut self);
    fn mark_protocol(&mut self);
    fn mark_enum(&mut self);
    fn mark_generator(&mut self);
    fn mark_property(&mut self);
    fn mark_classmethod(&mut self);
    fn mark_staticmethod(&mut self);
    fn push_attribute(&mut self, attribute: impl Into<String>);
}

impl PythonMetadataExt for SymbolMetadata {
    fn mark_exported(&mut self) {
        CommonMetadataExt::mark_exported(self);
    }

    fn mark_dataclass(&mut self) {
        self.is_dataclass = true;
    }

    fn mark_pydantic(&mut self) {
        self.is_pydantic = true;
    }

    fn mark_protocol(&mut self) {
        self.is_protocol = true;
    }

    fn mark_enum(&mut self) {
        self.is_enum = true;
    }

    fn mark_generator(&mut self) {
        CommonMetadataExt::mark_generator(self);
    }

    fn mark_property(&mut self) {
        self.is_property = true;
    }

    fn mark_classmethod(&mut self) {
        self.is_classmethod = true;
    }

    fn mark_staticmethod(&mut self) {
        self.is_staticmethod = true;
    }

    fn push_attribute(&mut self, attribute: impl Into<String>) {
        CommonMetadataExt::push_attribute(self, attribute);
    }
}
