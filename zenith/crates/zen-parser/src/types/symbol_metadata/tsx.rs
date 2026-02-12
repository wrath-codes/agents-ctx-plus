use super::SymbolMetadata;

pub trait TsxMetadataExt {
    fn set_component_directive(&mut self, directive: String);
    fn set_component(&mut self, value: bool);
    fn set_hook(&mut self, value: bool);
    fn set_hoc(&mut self, value: bool);
    fn set_forward_ref(&mut self, value: bool);
    fn set_memo(&mut self, value: bool);
    fn set_lazy(&mut self, value: bool);
    fn set_class_component(&mut self, value: bool);
    fn set_error_boundary(&mut self, value: bool);
    fn set_hooks_used(&mut self, hooks: Vec<String>);
    fn set_jsx_elements(&mut self, elements: Vec<String>);
    fn set_props_type_if_none(&mut self, props_type: Option<String>);
}

impl TsxMetadataExt for SymbolMetadata {
    fn set_component_directive(&mut self, directive: String) {
        self.component_directive = Some(directive);
    }

    fn set_component(&mut self, value: bool) {
        self.is_component = value;
    }

    fn set_hook(&mut self, value: bool) {
        self.is_hook = value;
    }

    fn set_hoc(&mut self, value: bool) {
        self.is_hoc = value;
    }

    fn set_forward_ref(&mut self, value: bool) {
        self.is_forward_ref = value;
    }

    fn set_memo(&mut self, value: bool) {
        self.is_memo = value;
    }

    fn set_lazy(&mut self, value: bool) {
        self.is_lazy = value;
    }

    fn set_class_component(&mut self, value: bool) {
        self.is_class_component = value;
    }

    fn set_error_boundary(&mut self, value: bool) {
        self.is_error_boundary = value;
    }

    fn set_hooks_used(&mut self, hooks: Vec<String>) {
        self.hooks_used = hooks;
    }

    fn set_jsx_elements(&mut self, elements: Vec<String>) {
        self.jsx_elements = elements;
    }

    fn set_props_type_if_none(&mut self, props_type: Option<String>) {
        if self.props_type.is_none() {
            self.props_type = props_type;
        }
    }
}
