//! Core data types for parsed symbols extracted from source code.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single extracted symbol from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedItem {
    pub kind: SymbolKind,
    pub name: String,
    pub signature: String,
    pub source: Option<String>,
    pub doc_comment: String,
    pub start_line: u32,
    pub end_line: u32,
    pub visibility: Visibility,
    pub metadata: SymbolMetadata,
}

/// The kind of extracted symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Interface,
    Class,
    TypeAlias,
    Const,
    Static,
    Macro,
    Module,
    Union,
    Component,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Class => "class",
            Self::TypeAlias => "type_alias",
            Self::Const => "const",
            Self::Static => "static",
            Self::Macro => "macro",
            Self::Module => "module",
            Self::Union => "union",
            Self::Component => "component",
        };
        write!(f, "{s}")
    }
}

/// Symbol visibility level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    PublicCrate,
    Private,
    Export,
    Protected,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Public => "public",
            Self::PublicCrate => "pub(crate)",
            Self::Private => "private",
            Self::Export => "export",
            Self::Protected => "protected",
        };
        write!(f, "{s}")
    }
}

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

/// Parsed documentation sections from doc comments/docstrings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocSections {
    pub errors: Option<String>,
    pub panics: Option<String>,
    pub safety: Option<String>,
    pub examples: Option<String>,
    pub args: HashMap<String, String>,
    pub returns: Option<String>,
    pub raises: HashMap<String, String>,
    pub yields: Option<String>,
    pub notes: Option<String>,
}
