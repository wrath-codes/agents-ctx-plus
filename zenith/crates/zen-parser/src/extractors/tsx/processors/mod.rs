//! TSX/React enrichment processors: function components, class components,
//! hooks, JSX detection, and React-specific metadata.

mod classes;
mod functions;

use crate::types::{ParsedItem, SymbolKind, TsxMetadataExt};

use super::tsx_helpers::detect_directive;

/// Metadata extracted from a function/arrow body.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct FnBody {
    pub start_line: u32,
    pub name: String,
    pub has_jsx: bool,
    pub hooks_used: Vec<String>,
    pub jsx_elements: Vec<String>,
    pub is_forward_ref: bool,
    pub is_memo: bool,
    pub is_lazy: bool,
    pub props_type: Option<String>,
    pub type_annotation: Option<String>,
}

/// Metadata extracted from a class declaration.
#[allow(clippy::struct_excessive_bools)]
pub(super) struct ClassInfo {
    pub start_line: u32,
    pub name: String,
    pub extends_react_component: bool,
    pub extends_pure_component: bool,
    pub has_derived_state_from_error: bool,
    pub has_component_did_catch: bool,
    pub jsx_elements: Vec<String>,
    pub props_type: Option<String>,
}

/// Enrich extracted items with React/JSX metadata.
pub(super) fn enrich_items<D: ast_grep_core::Doc>(
    root: &ast_grep_core::Node<D>,
    items: &mut [ParsedItem],
) {
    // Detect file-level directive ("use client" / "use server").
    let directive = detect_directive(root);

    // Build a lookup of function/arrow bodies for deeper analysis.
    let mut bodies: Vec<FnBody> = Vec::new();
    functions::collect_fn_bodies(root, &mut bodies);

    // Detect class components.
    let mut class_infos: Vec<ClassInfo> = Vec::new();
    classes::collect_class_components(root, &mut class_infos);

    for item in items.iter_mut() {
        enrich_item(item, &bodies, &class_infos);
        // Apply file-level directive to all items.
        if let Some(ref dir) = directive {
            item.metadata.set_component_directive(dir.clone());
        }
    }
}

fn enrich_item(item: &mut ParsedItem, bodies: &[FnBody], class_infos: &[ClassInfo]) {
    match item.kind {
        SymbolKind::Function
        | SymbolKind::Const
        | SymbolKind::Static
        | SymbolKind::Method
        | SymbolKind::Constructor
        | SymbolKind::Property => functions::enrich_fn_item(item, bodies),
        SymbolKind::Class => classes::enrich_class_item(item, class_infos),
        _ => {}
    }
}
