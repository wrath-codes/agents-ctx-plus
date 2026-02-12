use super::SymbolMetadata;

pub trait RustMetadataExt {
    fn mark_async(&mut self);
    fn mark_unsafe(&mut self);
    fn mark_pyo3(&mut self);
    fn set_abi(&mut self, abi: impl Into<String>);
    fn set_generics(&mut self, generics: impl Into<String>);
    fn set_where_clause(&mut self, where_clause: impl Into<String>);
    fn set_trait_target(&mut self, trait_name: impl Into<String>, for_type: impl Into<String>);
    fn push_lifetime(&mut self, lifetime: impl Into<String>);
    fn push_associated_type(&mut self, associated_type: impl Into<String>);
}

impl RustMetadataExt for SymbolMetadata {
    fn mark_async(&mut self) {
        self.is_async = true;
    }

    fn mark_unsafe(&mut self) {
        self.is_unsafe = true;
    }

    fn mark_pyo3(&mut self) {
        self.is_pyo3 = true;
    }

    fn set_abi(&mut self, abi: impl Into<String>) {
        self.abi = Some(abi.into());
    }

    fn set_generics(&mut self, generics: impl Into<String>) {
        self.generics = Some(generics.into());
    }

    fn set_where_clause(&mut self, where_clause: impl Into<String>) {
        self.where_clause = Some(where_clause.into());
    }

    fn set_trait_target(&mut self, trait_name: impl Into<String>, for_type: impl Into<String>) {
        self.trait_name = Some(trait_name.into());
        self.for_type = Some(for_type.into());
    }

    fn push_lifetime(&mut self, lifetime: impl Into<String>) {
        let value = lifetime.into();
        if !value.is_empty() && !self.lifetimes.contains(&value) {
            self.lifetimes.push(value);
        }
    }

    fn push_associated_type(&mut self, associated_type: impl Into<String>) {
        let value = associated_type.into();
        if !value.is_empty() && !self.associated_types.contains(&value) {
            self.associated_types.push(value);
        }
    }
}
