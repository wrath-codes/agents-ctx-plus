//! Core data types for parsed symbols extracted from source code.

mod doc_sections;
mod parsed_item;
mod symbol_kind;
mod visibility;

mod symbol_metadata;

pub use doc_sections::DocSections;
pub use parsed_item::ParsedItem;
pub use symbol_kind::SymbolKind;
pub use symbol_metadata::SymbolMetadata;
pub use symbol_metadata::bash::BashMetadataExt;
pub use symbol_metadata::c::CMetadataExt;
pub use symbol_metadata::common::CommonMetadataExt;
pub use symbol_metadata::cpp::CppMetadataExt;
pub use symbol_metadata::css::CssMetadataExt;
pub use symbol_metadata::elixir::ElixirMetadataExt;
pub use symbol_metadata::go::GoMetadataExt;
pub use symbol_metadata::html::HtmlMetadataExt;
pub use symbol_metadata::javascript::JavaScriptMetadataExt;
pub use symbol_metadata::python::PythonMetadataExt;
pub use symbol_metadata::rust::RustMetadataExt;
pub use symbol_metadata::tsx::TsxMetadataExt;
pub use symbol_metadata::typescript::TypeScriptMetadataExt;
pub use visibility::Visibility;
