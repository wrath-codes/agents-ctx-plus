//! Markdown extractor powered by custom `tree-sitter-md` language support.

use ast_grep_core::matcher::KindMatcher;

use crate::types::ParsedItem;

#[path = "../markdown/helpers.rs"]
mod markdown_helpers;
#[path = "../markdown/processors.rs"]
mod processors;

#[derive(Clone)]
struct HeadingContext {
    start_line: u32,
    level: u8,
    path: String,
}

fn owner_path_for_line(headings: &[HeadingContext], line: u32) -> String {
    headings
        .iter()
        .rev()
        .find(|h| h.start_line <= line)
        .map_or_else(|| "$".to_string(), |h| h.path.clone())
}

fn set_owner(item: &mut ParsedItem, owner: &str) {
    item.metadata.owner_name = Some(owner.to_string());
    item.metadata.owner_kind = Some(crate::types::SymbolKind::Module);
    item.metadata
        .attributes
        .push(format!("md:owner_path:{owner}"));
}

/// Extract significant markdown symbols from a document.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = vec![processors::root_item(&root.root())];
    let lang = root.root().lang().clone();

    let mut heading_nodes: Vec<_> = root
        .root()
        .find_all(KindMatcher::new("atx_heading", lang.clone()))
        .collect();
    heading_nodes.extend(
        root.root()
            .find_all(KindMatcher::new("setext_heading", lang.clone())),
    );
    heading_nodes.sort_by_key(|n| n.start_pos().line());

    let mut heading_ctx = Vec::<HeadingContext>::new();
    let mut heading_stack = Vec::<HeadingContext>::new();
    for node in heading_nodes {
        let mut item = processors::heading_item(&node);
        let raw = node.text().to_string();
        let level = markdown_helpers::heading_level(&raw).unwrap_or(1);

        while heading_stack.last().is_some_and(|h| h.level >= level) {
            heading_stack.pop();
        }

        if let Some(parent) = heading_stack.last() {
            set_owner(&mut item, &parent.path);
        }

        let path = if let Some(parent) = heading_stack.last() {
            format!("{}/{}", parent.path, item.name)
        } else {
            item.name.clone()
        };
        item.metadata.attributes.push(format!("md:path:{path}"));

        let next = HeadingContext {
            start_line: item.start_line,
            level,
            path,
        };
        heading_stack.push(next.clone());
        heading_ctx.push(next);
        items.push(item);
    }

    for kind in [
        "fenced_code_block",
        "list",
        "pipe_table",
        "link_reference_definition",
        "thematic_break",
        "minus_metadata",
        "plus_metadata",
    ] {
        for node in root.root().find_all(KindMatcher::new(kind, lang.clone())) {
            let mut item = match kind {
                "fenced_code_block" => processors::code_fence_item(&node),
                "list" => processors::list_item(&node),
                "pipe_table" => processors::table_item(&node),
                "link_reference_definition" => processors::link_reference_item(&node),
                "thematic_break" => processors::thematic_break_item(&node),
                "minus_metadata" => processors::frontmatter_item(&node, "yaml"),
                "plus_metadata" => processors::frontmatter_item(&node, "toml"),
                _ => unreachable!("unsupported markdown kind: {kind}"),
            };
            let line = item.start_line;
            let owner = owner_path_for_line(&heading_ctx, line);
            set_owner(&mut item, &owner);
            items.push(item);
        }
    }

    for node in root
        .root()
        .find_all(KindMatcher::new("paragraph", lang.clone()))
    {
        for mut item in processors::inline_items_from_node(&node) {
            let line = item.start_line;
            let owner = owner_path_for_line(&heading_ctx, line);
            set_owner(&mut item, &owner);
            items.push(item);
        }
    }

    for node in root
        .root()
        .find_all(KindMatcher::new("atx_heading", lang.clone()))
    {
        for mut item in processors::inline_items_from_node(&node) {
            let line = item.start_line;
            let owner = owner_path_for_line(&heading_ctx, line);
            set_owner(&mut item, &owner);
            items.push(item);
        }
    }

    for node in root
        .root()
        .find_all(KindMatcher::new("setext_heading", lang))
    {
        for mut item in processors::inline_items_from_node(&node) {
            let line = item.start_line;
            let owner = owner_path_for_line(&heading_ctx, line);
            set_owner(&mut item, &owner);
            items.push(item);
        }
    }

    Ok(items)
}

#[cfg(test)]
#[path = "../markdown/tests/mod.rs"]
mod tests;
