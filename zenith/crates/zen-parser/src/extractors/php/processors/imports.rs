use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::php_helpers;
use super::build_item;

pub(super) fn process_namespace_use_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Vec<ParsedItem> {
    let mut items = Vec::new();

    let declaration_type = node.field("type").map(|t| t.text().to_string());
    let namespace_prefix = node
        .children()
        .find(|c| c.kind().as_ref() == "namespace_name")
        .map(|prefix| prefix.text().to_string());

    let clauses: Vec<_> = node.field("body").map_or_else(
        || {
            node.children()
                .filter(|c| c.kind().as_ref() == "namespace_use_clause")
                .collect()
        },
        |group| {
            group
                .children()
                .filter(|c| c.kind().as_ref() == "namespace_use_clause")
                .collect()
        },
    );

    if clauses.is_empty() {
        let name = node.text().trim().trim_end_matches(';').to_string();
        if !name.is_empty() {
            items.push(build_item(
                node,
                SymbolKind::Module,
                name,
                Visibility::Public,
                SymbolMetadata::default(),
                String::new(),
            ));
        }
        return items;
    }

    for clause in clauses {
        let import_name = clause
            .children()
            .find(|c| matches!(c.kind().as_ref(), "name" | "qualified_name"))
            .map(|name| name.text().to_string());

        let Some(base_name) = import_name else {
            continue;
        };

        let full_name = if let Some(prefix) = &namespace_prefix {
            if base_name.contains('\\') {
                base_name
            } else {
                format!("{prefix}\\{base_name}")
            }
        } else {
            base_name
        };
        let full_name = php_helpers::normalize_php_name(&full_name);

        let alias = clause.field("alias").map(|a| a.text().to_string());
        let kind = clause
            .field("type")
            .map(|k| k.text().to_string())
            .or_else(|| declaration_type.clone())
            .unwrap_or_else(|| "class".to_string());

        let display_name = alias
            .as_ref()
            .map_or_else(|| full_name.clone(), |a| format!("{full_name} as {a}"));

        let mut metadata = SymbolMetadata {
            attributes: vec![format!("import_kind:{kind}")],
            return_type: Some(full_name),
            ..Default::default()
        };
        if let Some(alias_name) = &alias {
            metadata
                .attributes
                .push(format!("import_alias:{alias_name}"));
        }

        items.push(build_item(
            &clause,
            SymbolKind::Module,
            display_name,
            Visibility::Public,
            metadata,
            String::new(),
        ));
    }

    items
}
