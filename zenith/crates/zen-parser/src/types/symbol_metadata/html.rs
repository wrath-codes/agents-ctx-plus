use super::SymbolMetadata;

pub trait HtmlMetadataExt {
    fn set_tag_name(&mut self, tag_name: impl Into<String>);
    fn set_element_id(&mut self, element_id: Option<String>);
    fn set_class_names(&mut self, class_names: Vec<String>);
    fn set_html_attributes(&mut self, html_attributes: Vec<(String, Option<String>)>);
    fn set_custom_element(&mut self, is_custom_element: bool);
    fn set_self_closing(&mut self, is_self_closing: bool);
}

impl HtmlMetadataExt for SymbolMetadata {
    fn set_tag_name(&mut self, tag_name: impl Into<String>) {
        self.tag_name = Some(tag_name.into());
    }

    fn set_element_id(&mut self, element_id: Option<String>) {
        self.element_id = element_id;
    }

    fn set_class_names(&mut self, class_names: Vec<String>) {
        self.class_names = class_names;
    }

    fn set_html_attributes(&mut self, html_attributes: Vec<(String, Option<String>)>) {
        self.html_attributes = html_attributes;
    }

    fn set_custom_element(&mut self, is_custom_element: bool) {
        self.is_custom_element = is_custom_element;
    }

    fn set_self_closing(&mut self, is_self_closing: bool) {
        self.is_self_closing = is_self_closing;
    }
}
