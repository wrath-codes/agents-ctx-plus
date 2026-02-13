use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::super::php_helpers;
use super::{build_item, types};

pub(super) fn process_inline_symbol<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    match node.kind().as_ref() {
        "anonymous_function" => {
            let mut metadata = SymbolMetadata {
                parameters: php_helpers::extract_parameter_descriptors(node),
                return_type: types::normalize_type_node(node.field("return_type")),
                attributes: php_helpers::extract_attributes(node),
                ..Default::default()
            };
            let (origin, alias) = php_helpers::callable_context(node);
            metadata
                .attributes
                .push(format!("callable_origin:{origin}"));
            if let Some(alias) = alias {
                metadata.attributes.push(format!("callable_alias:{alias}"));
            }

            if let Some(use_clause) = node
                .children()
                .find(|c| c.kind().as_ref() == "anonymous_function_use_clause")
            {
                metadata.attributes.extend(
                    use_clause
                        .children()
                        .filter(|c| c.kind().as_ref() == "variable_name")
                        .map(|c| format!("closure_use:{}", c.text())),
                );
            }

            Some(build_item(
                node,
                SymbolKind::Function,
                php_helpers::synthetic_name("closure", node),
                Visibility::Private,
                metadata,
                php_helpers::extract_doc_before(node),
            ))
        }
        "arrow_function" => {
            let (origin, alias) = php_helpers::callable_context(node);
            let metadata = SymbolMetadata {
                parameters: php_helpers::extract_parameter_descriptors(node),
                return_type: types::normalize_type_node(node.field("return_type")),
                attributes: {
                    let mut attrs = php_helpers::extract_attributes(node);
                    attrs.push(format!("callable_origin:{origin}"));
                    if let Some(alias) = alias {
                        attrs.push(format!("callable_alias:{alias}"));
                    }
                    attrs
                },
                ..Default::default()
            };
            Some(build_item(
                node,
                SymbolKind::Function,
                php_helpers::synthetic_name("arrow", node),
                Visibility::Private,
                metadata,
                String::new(),
            ))
        }
        "object_creation_expression" => {
            let anonymous = node
                .children()
                .find(|c| c.kind().as_ref() == "anonymous_class")?;
            let metadata = SymbolMetadata {
                base_classes: php_helpers::extract_type_bases(&anonymous),
                attributes: php_helpers::extract_attributes(&anonymous),
                ..Default::default()
            };
            Some(build_item(
                &anonymous,
                SymbolKind::Class,
                php_helpers::synthetic_name("anonymous_class", node),
                Visibility::Private,
                metadata,
                php_helpers::extract_doc_before(node),
            ))
        }
        _ => None,
    }
}
