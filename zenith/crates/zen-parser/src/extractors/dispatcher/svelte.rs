//! Svelte extractor powered by custom `tree-sitter-svelte-next` support.

use ast_grep_core::matcher::KindMatcher;

use crate::types::ParsedItem;

#[path = "../svelte/helpers.rs"]
mod svelte_helpers;
#[path = "../svelte/processors.rs"]
mod processors;

/// Extract significant Svelte symbols from a document.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = vec![processors::root_item(&root.root())];
    let lang = root.root().lang().clone();

    for node in root
        .root()
        .find_all(KindMatcher::new("script_element", lang.clone()))
    {
        let script = processors::script_item(&node);
        let script_name = script.name.clone();
        items.push(script);
        items.extend(processors::script_api_items(&node, &script_name));
    }

    for node in root
        .root()
        .find_all(KindMatcher::new("style_element", lang.clone()))
    {
        items.push(processors::style_item(&node));
    }

    for node in root
        .root()
        .find_all(KindMatcher::new("element", lang.clone()))
    {
        if let Some(item) = processors::element_item(&node) {
            let owner = item.name.clone();
            items.push(item);
            items.extend(processors::directive_attr_items(&node, &owner));
        }
    }

    for (kind, name) in [
        ("if_statement", "if"),
        ("each_statement", "each"),
        ("await_statement", "await"),
        ("key_statement", "key"),
        ("snippet_statement", "snippet"),
    ] {
        for node in root.root().find_all(KindMatcher::new(kind, lang.clone())) {
            items.push(processors::block_item(&node, name, kind));
        }
    }

    for (kind, name) in [
        ("expression_tag", "expression_tag"),
        ("render_tag", "render_tag"),
        ("html_tag", "html_tag"),
        ("const_tag", "const_tag"),
        ("debug_tag", "debug_tag"),
    ] {
        for node in root.root().find_all(KindMatcher::new(kind, lang.clone())) {
            items.push(processors::tag_item(&node, name));
        }
    }

    items.extend(processors::duplicate_id_items(&root.root()));
    processors::link_snippet_render_refs(&mut items);

    Ok(items)
}

#[cfg(test)]
#[path = "../svelte/tests/mod.rs"]
mod tests;
