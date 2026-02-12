//! Enrichment pass: annotates items with constexpr, noexcept, scoped enum,
//! trailing return type, C++11 attributes, and promotes constexpr/constinit
//! variables to Const.

use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind};

pub(super) fn enrich_items<D: ast_grep_core::Doc>(root: &Node<D>, items: &mut Vec<ParsedItem>) {
    enrich_recursive(root, items);
}

fn enrich_recursive<D: ast_grep_core::Doc>(node: &Node<D>, items: &mut Vec<ParsedItem>) {
    let kind = node.kind();
    let start_line = node.start_pos().line() as u32 + 1;

    match kind.as_ref() {
        "enum_specifier" => {
            // Detect scoped enum (enum class)
            let is_scoped = node.children().any(|c| c.kind().as_ref() == "class");
            if is_scoped
                && let Some(item) = items
                    .iter_mut()
                    .find(|i| i.kind == SymbolKind::Enum && i.start_line == start_line)
                && !item
                    .metadata
                    .attributes
                    .contains(&"scoped_enum".to_string())
            {
                item.metadata.attributes.push("scoped_enum".to_string());
            }
        }
        "function_definition" | "declaration" => {
            let children: Vec<_> = node.children().collect();
            let mut cpp_attrs = Vec::new();

            for child in &children {
                match child.kind().as_ref() {
                    "type_qualifier" => {
                        let t = child.text();
                        match t.as_ref() {
                            "constexpr" => cpp_attrs.push("constexpr".to_string()),
                            "consteval" => cpp_attrs.push("consteval".to_string()),
                            "constinit" => cpp_attrs.push("constinit".to_string()),
                            _ => {}
                        }
                    }
                    "function_declarator" => {
                        let fc: Vec<_> = child.children().collect();
                        for f in &fc {
                            if f.kind().as_ref() == "noexcept" {
                                cpp_attrs.push("noexcept".to_string());
                            }
                            if f.kind().as_ref() == "trailing_return_type" {
                                let rt_text = f
                                    .text()
                                    .to_string()
                                    .trim_start_matches("->")
                                    .trim()
                                    .to_string();
                                if let Some(item) = items.iter_mut().find(|i| {
                                    i.start_line == start_line
                                        && (i.kind == SymbolKind::Function
                                            || i.kind == SymbolKind::Const
                                            || i.kind == SymbolKind::Static)
                                }) {
                                    item.metadata.return_type = Some(rt_text);
                                }
                            }
                        }
                    }
                    "placeholder_type_specifier" => {
                        cpp_attrs.push("auto".to_string());
                    }
                    _ => {}
                }
            }

            if !cpp_attrs.is_empty()
                && let Some(item) = items.iter_mut().find(|i| i.start_line == start_line)
            {
                for attr in &cpp_attrs {
                    if !item.metadata.attributes.contains(attr) {
                        item.metadata.attributes.push(attr.clone());
                    }
                }
                // constexpr/consteval/constinit variables should be Const
                if (item.kind == SymbolKind::Static)
                    && (cpp_attrs.contains(&"constexpr".to_string())
                        || cpp_attrs.contains(&"constinit".to_string()))
                {
                    item.kind = SymbolKind::Const;
                }
            }
        }
        "attributed_declaration" | "attributed_statement" => {
            // Annotate items with C++11 [[...]] attributes from this wrapper
            let attr_start_line = start_line;
            for attr_child in node.children() {
                if attr_child.kind().as_ref() == "attribute_declaration" {
                    let attr_text = attr_child.text().to_string();
                    if let Some(item) = items.iter_mut().find(|i| i.start_line == attr_start_line)
                        && !item.metadata.attributes.contains(&attr_text)
                    {
                        item.metadata.attributes.push(attr_text);
                    }
                }
            }
        }
        _ => {}
    }

    let children: Vec<_> = node.children().collect();
    for child in &children {
        enrich_recursive(child, items);
    }
}
