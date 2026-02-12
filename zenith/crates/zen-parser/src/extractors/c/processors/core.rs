//! Core types and qualifier detection for C extraction.

use ast_grep_core::Node;

use crate::types::{CMetadataExt, SymbolKind, SymbolMetadata, Visibility};

/// Storage class specifiers and type qualifiers on a declaration.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct Qualifiers {
    pub(super) is_static: bool,
    pub(super) is_inline: bool,
    pub(super) is_extern: bool,
    pub(super) is_const: bool,
    pub(super) is_volatile: bool,
    pub(super) is_register: bool,
    /// GCC `__attribute__((…))` texts.
    pub(super) gcc_attributes: Vec<String>,
    /// C11 qualifiers like `_Noreturn`, `_Atomic`, `restrict`, `_Alignas(…)`.
    pub(super) c11_attrs: Vec<String>,
}

pub(super) fn detect_qualifiers<D: ast_grep_core::Doc>(node: &Node<D>) -> Qualifiers {
    let mut q = Qualifiers {
        is_static: false,
        is_inline: false,
        is_extern: false,
        is_const: false,
        is_volatile: false,
        is_register: false,
        gcc_attributes: Vec::new(),
        c11_attrs: Vec::new(),
    };
    let children: Vec<_> = node.children().collect();
    for child in &children {
        match child.kind().as_ref() {
            "storage_class_specifier" => {
                let text = child.text();
                match text.as_ref() {
                    "static" => q.is_static = true,
                    "inline" => q.is_inline = true,
                    "extern" => q.is_extern = true,
                    "register" => q.is_register = true,
                    _ => {}
                }
            }
            "type_qualifier" => {
                let text = child.text();
                match text.as_ref() {
                    "const" => q.is_const = true,
                    "volatile" => q.is_volatile = true,
                    "_Noreturn" | "_Atomic" | "restrict" => {
                        q.c11_attrs.push(text.to_string());
                    }
                    other if other.starts_with("_Alignas") => {
                        q.c11_attrs.push(text.to_string());
                    }
                    _ => {}
                }
            }
            "attribute_specifier" => {
                q.gcc_attributes.push(child.text().to_string());
            }
            _ => {}
        }
    }
    q
}

/// Determine visibility from qualifiers.
pub(super) const fn visibility_from_qualifiers(q: &Qualifiers) -> Visibility {
    if q.is_static {
        Visibility::Private
    } else {
        Visibility::Public
    }
}

/// Build attribute list from qualifiers.
pub(super) fn attributes_from_qualifiers(q: &Qualifiers) -> Vec<String> {
    let mut metadata = SymbolMetadata::default();
    if q.is_static {
        metadata.push_attribute("static");
    }
    if q.is_inline {
        metadata.push_attribute("inline");
    }
    if q.is_extern {
        metadata.push_attribute("extern");
    }
    if q.is_const {
        metadata.push_attribute("const");
    }
    if q.is_volatile {
        metadata.push_attribute("volatile");
    }
    if q.is_register {
        metadata.push_attribute("register");
    }
    for attr in &q.gcc_attributes {
        metadata.push_attribute(attr.clone());
    }
    for eq in &q.c11_attrs {
        metadata.push_attribute(eq.clone());
    }
    metadata.attributes
}

/// Classify a variable by its qualifiers into (kind, visibility).
pub(super) const fn classify_variable(q: &Qualifiers) -> (SymbolKind, Visibility) {
    if q.is_const {
        (SymbolKind::Const, visibility_from_qualifiers(q))
    } else {
        (SymbolKind::Static, visibility_from_qualifiers(q))
    }
}
