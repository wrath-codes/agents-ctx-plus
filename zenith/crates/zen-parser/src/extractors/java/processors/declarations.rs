use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::java_helpers;
use super::build_item;

pub(super) fn process_module_like<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let signature = crate::extractors::helpers::extract_signature(node);
    let name = match node.kind().as_ref() {
        "package_declaration" => signature
            .trim_start_matches("package")
            .trim()
            .trim_end_matches(';')
            .to_string(),
        "import_declaration" => signature
            .trim_start_matches("import")
            .trim()
            .trim_end_matches(';')
            .to_string(),
        "module_declaration" => signature
            .split_whitespace()
            .find(|part| *part != "open" && *part != "module")
            .map_or_else(String::new, ToString::to_string),
        _ => return None,
    };

    if name.is_empty() {
        return None;
    }

    Some(build_item(
        node,
        SymbolKind::Module,
        name,
        Visibility::Public,
        SymbolMetadata::default(),
        java_helpers::extract_javadoc_before(node),
    ))
}

pub(super) fn process_type_declaration<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let kind = match node.kind().as_ref() {
        "class_declaration" => SymbolKind::Class,
        "interface_declaration" | "annotation_type_declaration" => SymbolKind::Interface,
        "enum_declaration" => SymbolKind::Enum,
        "record_declaration" => SymbolKind::Struct,
        _ => return Vec::new(),
    };

    let Some(name) = node.field("name").map(|name| name.text().to_string()) else {
        return Vec::new();
    };
    let modifiers = java_helpers::extract_modifiers(node);
    let visibility = java_helpers::visibility_from_modifiers(&modifiers);

    let mut metadata = SymbolMetadata {
        type_parameters: node
            .field("type_parameters")
            .map(|params| params.text().to_string()),
        base_classes: java_helpers::extract_base_types(node),
        ..Default::default()
    };

    if kind == SymbolKind::Enum {
        metadata.variants = java_helpers::extract_enum_variants(node);
    }

    if kind == SymbolKind::Struct {
        metadata.fields = java_helpers::extract_record_components(node)
            .iter()
            .map(|(field_name, _)| field_name.clone())
            .collect();
    }

    let mut items = vec![build_item(
        node,
        kind,
        name.clone(),
        visibility,
        metadata,
        java_helpers::extract_javadoc_before(node),
    )];

    if kind == SymbolKind::Struct {
        items.extend(
            java_helpers::extract_record_components(node)
                .into_iter()
                .map(|(field_name, field_type)| {
                    let metadata = SymbolMetadata {
                        owner_name: Some(name.clone()),
                        owner_kind: Some(SymbolKind::Struct),
                        return_type: field_type,
                        ..Default::default()
                    };
                    ParsedItem {
                        kind: SymbolKind::Field,
                        name: field_name.clone(),
                        signature: field_name,
                        source: None,
                        doc_comment: String::new(),
                        start_line: node.start_pos().line() as u32 + 1,
                        end_line: node.end_pos().line() as u32 + 1,
                        visibility: Visibility::Private,
                        metadata,
                    }
                }),
        );
    }

    items
}

pub(super) fn process_module_directive<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    let kind = node.kind();
    if !kind.as_ref().ends_with("_module_directive") {
        return None;
    }

    let signature = crate::extractors::helpers::extract_signature(node);
    if signature.is_empty() {
        return None;
    }

    let mut metadata = SymbolMetadata::default();
    if let Some(parts) = java_helpers::extract_module_directive_parts(node) {
        metadata
            .attributes
            .push(format!("module_directive:{}", parts.directive));
        metadata.return_type = parts.subject;
        metadata.parameters = parts.targets;
        metadata.attributes.extend(
            parts
                .modifiers
                .into_iter()
                .map(|modifier| format!("module_modifier:{modifier}")),
        );
    }

    Some(build_item(
        node,
        SymbolKind::Module,
        signature,
        Visibility::Public,
        metadata,
        java_helpers::extract_javadoc_before(node),
    ))
}
