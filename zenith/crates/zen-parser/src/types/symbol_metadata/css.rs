use super::SymbolMetadata;

pub trait CssMetadataExt {
    fn set_selector(&mut self, selector: impl Into<String>);
    fn mark_custom_property(&mut self);
    fn set_at_rule_name(&mut self, at_rule_name: impl Into<String>);
    fn set_media_query(&mut self, media_query: impl Into<String>);
    fn set_css_properties(&mut self, css_properties: Vec<String>);
}

impl CssMetadataExt for SymbolMetadata {
    fn set_selector(&mut self, selector: impl Into<String>) {
        self.selector = Some(selector.into());
    }

    fn mark_custom_property(&mut self) {
        self.is_custom_property = true;
    }

    fn set_at_rule_name(&mut self, at_rule_name: impl Into<String>) {
        self.at_rule_name = Some(at_rule_name.into());
    }

    fn set_media_query(&mut self, media_query: impl Into<String>) {
        self.media_query = Some(media_query.into());
    }

    fn set_css_properties(&mut self, css_properties: Vec<String>) {
        self.css_properties = css_properties;
    }
}
