use serde::{Deserialize, Serialize};

use super::DocSections;

pub mod bash;
pub mod c;
pub mod common;
pub mod cpp;
pub mod css;
pub mod elixir;
pub mod go;
pub mod html;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod tsx;
pub mod typescript;

/// Language-specific metadata attached to a `ParsedItem`.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolMetadata {
    // Common
    pub is_async: bool,
    pub is_unsafe: bool,
    pub return_type: Option<String>,
    pub generics: Option<String>,
    pub attributes: Vec<String>,
    pub parameters: Vec<String>,

    // Rust-specific
    pub lifetimes: Vec<String>,
    pub where_clause: Option<String>,
    pub trait_name: Option<String>,
    pub for_type: Option<String>,
    pub associated_types: Vec<String>,
    pub abi: Option<String>,
    pub is_pyo3: bool,

    // Enum/Struct members
    pub variants: Vec<String>,
    pub fields: Vec<String>,
    pub methods: Vec<String>,

    // Python-specific
    pub is_generator: bool,
    pub is_property: bool,
    pub is_classmethod: bool,
    pub is_staticmethod: bool,
    pub is_dataclass: bool,
    pub is_pydantic: bool,
    pub is_protocol: bool,
    pub is_enum: bool,
    pub base_classes: Vec<String>,
    pub decorators: Vec<String>,

    // TypeScript-specific
    pub is_exported: bool,
    pub is_default_export: bool,
    pub type_parameters: Option<String>,
    pub implements: Vec<String>,

    // Documentation
    pub doc_sections: DocSections,

    // Error detection
    pub is_error_type: bool,
    pub returns_result: bool,

    // HTML-specific
    pub tag_name: Option<String>,
    pub element_id: Option<String>,
    pub class_names: Vec<String>,
    pub html_attributes: Vec<(String, Option<String>)>,
    pub is_custom_element: bool,
    pub is_self_closing: bool,

    // CSS-specific
    pub selector: Option<String>,
    pub media_query: Option<String>,
    pub at_rule_name: Option<String>,
    pub css_properties: Vec<String>,
    pub is_custom_property: bool,

    // TSX/React-specific
    pub is_component: bool,
    pub is_hook: bool,
    pub is_hoc: bool,
    pub is_forward_ref: bool,
    pub is_memo: bool,
    pub is_lazy: bool,
    pub is_class_component: bool,
    pub is_error_boundary: bool,
    pub component_directive: Option<String>,
    pub props_type: Option<String>,
    pub hooks_used: Vec<String>,
    pub jsx_elements: Vec<String>,
}
