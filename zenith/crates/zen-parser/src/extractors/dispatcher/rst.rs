//! reStructuredText extractor powered by custom `tree-sitter-rst` support.

use ast_grep_core::matcher::KindMatcher;
use std::collections::HashMap;

use crate::types::ParsedItem;

#[path = "../rst/processors.rs"]
mod processors;
#[path = "../rst/helpers.rs"]
mod rst_helpers;

#[derive(Clone)]
struct SectionContext {
    start_line: u32,
    end_line: u32,
    level: u8,
    path: String,
}

fn owner_path_for_line(sections: &[SectionContext], line: u32) -> String {
    sections
        .iter()
        .rev()
        .find(|s| s.start_line <= line && line <= s.end_line)
        .map_or_else(|| "$".to_string(), |s| s.path.clone())
}

fn set_owner(item: &mut ParsedItem, owner: &str) {
    item.metadata.owner_name = Some(owner.to_string());
    item.metadata.owner_kind = Some(crate::types::SymbolKind::Module);
    item.metadata
        .attributes
        .push(format!("rst:owner_path:{owner}"));
}

fn attr_value<'a>(item: &'a ParsedItem, prefix: &str) -> Option<&'a str> {
    item.metadata
        .attributes
        .iter()
        .find_map(|a| a.strip_prefix(prefix))
}

fn resolve_references(items: &mut [ParsedItem]) {
    let mut targets: HashMap<String, String> = HashMap::new();
    let mut footnotes: HashMap<String, String> = HashMap::new();
    let mut citations: HashMap<String, String> = HashMap::new();
    let mut substitutions: HashMap<String, String> = HashMap::new();

    for item in items.iter_mut() {
        if let Some(label) = attr_value(item, "rst:label:target:")
            && let Some(prev) = targets.insert(label.to_string(), item.name.clone())
        {
            item.metadata
                .attributes
                .push(format!("rst:duplicate_target_label:{label}"));
            item.metadata
                .attributes
                .push(format!("rst:duplicate_target_previous:{prev}"));
        }
        if let Some(label) = attr_value(item, "rst:label:footnote:") {
            footnotes.insert(label.to_string(), item.name.clone());
        }
        if let Some(label) = attr_value(item, "rst:label:citation:") {
            citations.insert(label.to_string(), item.name.clone());
        }
        if let Some(label) = attr_value(item, "rst:label:substitution:") {
            substitutions.insert(label.to_string(), item.name.clone());
        }
    }

    for item in items.iter_mut() {
        if let Some(label) = attr_value(item, "rst:ref_label:") {
            if let Some(target) = targets.get(label) {
                item.metadata
                    .attributes
                    .push(format!("rst:ref_target:{target}"));
            } else {
                item.metadata
                    .attributes
                    .push(format!("rst:broken_reference:{label}"));
            }
            continue;
        }

        if let Some(label) = attr_value(item, "rst:ref_label:footnote:") {
            if let Some(target) = footnotes.get(label) {
                item.metadata
                    .attributes
                    .push(format!("rst:ref_target:{target}"));
            } else {
                item.metadata
                    .attributes
                    .push(format!("rst:broken_footnote_reference:{label}"));
            }
            continue;
        }

        if let Some(label) = attr_value(item, "rst:ref_label:citation:") {
            if let Some(target) = citations.get(label) {
                item.metadata
                    .attributes
                    .push(format!("rst:ref_target:{target}"));
            } else {
                item.metadata
                    .attributes
                    .push(format!("rst:broken_citation_reference:{label}"));
            }
            continue;
        }

        if let Some(label) = attr_value(item, "rst:ref_label:substitution:") {
            if let Some(target) = substitutions.get(label) {
                item.metadata
                    .attributes
                    .push(format!("rst:ref_target:{target}"));
            } else {
                item.metadata
                    .attributes
                    .push(format!("rst:broken_substitution_reference:{label}"));
            }
        }
    }
}

/// Extract significant RST symbols from a document.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
#[allow(clippy::too_many_lines)]
pub fn extract<D: ast_grep_core::Doc>(
    root: &ast_grep_core::AstGrep<D>,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = vec![processors::root_item(&root.root())];
    let lang = root.root().lang().clone();

    let mut section_nodes: Vec<_> = root
        .root()
        .find_all(KindMatcher::new("section", lang.clone()))
        .collect();
    section_nodes.sort_by_key(|n| n.start_pos().line());

    let mut stack: Vec<SectionContext> = Vec::new();
    let mut section_ctx: Vec<SectionContext> = Vec::new();

    for node in section_nodes {
        let level = rst_helpers::section_level_from_text(&node.text());
        let mut item = processors::section_item(&node, level);
        let start = item.start_line;
        let end = item.end_line;

        while stack
            .last()
            .is_some_and(|s| s.level >= level || start > s.end_line)
        {
            stack.pop();
        }

        let owner_base = stack.last().map_or_else(
            || section_ctx.last().map(|s| s.path.clone()),
            |parent| Some(parent.path.clone()),
        );

        if let Some(owner) = &owner_base {
            set_owner(&mut item, owner);
        }

        let path = if let Some(owner) = owner_base {
            format!("{owner}/{}", item.name)
        } else {
            item.name.clone()
        };
        item.metadata.attributes.push(format!("rst:path:{path}"));

        let ctx = SectionContext {
            start_line: start,
            end_line: end,
            level,
            path,
        };
        stack.push(ctx.clone());
        section_ctx.push(ctx);
        items.push(item);
    }

    for kind in [
        "directive",
        "target",
        "footnote",
        "citation",
        "substitution_definition",
        "comment",
        "bullet_list",
        "enumerated_list",
        "definition_list",
        "field_list",
        "literal_block",
        "doctest_block",
        "line_block",
        "block_quote",
        "transition",
        "field",
        "field_body",
        "content",
        "options",
        "definition",
        "classifier",
    ] {
        for node in root.root().find_all(KindMatcher::new(kind, lang.clone())) {
            let mut item = match kind {
                "directive" => processors::directive_item(&node),
                "target" => processors::target_item(&node),
                "footnote" => processors::footnote_item(&node),
                "citation" => processors::citation_item(&node),
                "substitution_definition" => processors::substitution_definition_item(&node),
                "comment" => processors::comment_item(&node),
                "bullet_list" => processors::list_item(&node, "bullet_list"),
                "enumerated_list" => processors::list_item(&node, "enumerated_list"),
                "definition_list" => processors::list_item(&node, "definition_list"),
                "field_list" => processors::list_item(&node, "field_list"),
                "literal_block" => processors::block_item(&node, "literal_block"),
                "doctest_block" => processors::block_item(&node, "doctest_block"),
                "line_block" => processors::block_item(&node, "line_block"),
                "block_quote" => processors::block_item(&node, "block_quote"),
                "transition" => processors::block_item(&node, "transition"),
                "field" => processors::generic_named_item(&node, "field"),
                "field_body" => processors::generic_named_item(&node, "field_body"),
                "content" => processors::generic_named_item(&node, "content"),
                "options" => processors::generic_named_item(&node, "options"),
                "definition" => processors::generic_named_item(&node, "definition"),
                "classifier" => processors::generic_named_item(&node, "classifier"),
                _ => unreachable!("unsupported rst kind: {kind}"),
            };

            let owner = owner_path_for_line(&section_ctx, item.start_line);
            set_owner(&mut item, &owner);
            items.push(item);
        }
    }

    for node_kind in ["paragraph", "title", "term", "field_name", "line"] {
        for node in root
            .root()
            .find_all(KindMatcher::new(node_kind, lang.clone()))
        {
            let owner = owner_path_for_line(&section_ctx, node.start_pos().line() as u32 + 1);
            for inline_kind in [
                "reference",
                "standalone_hyperlink",
                "footnote_reference",
                "citation_reference",
                "literal",
                "emphasis",
                "strong",
                "interpreted_text",
                "inline_target",
                "substitution_reference",
            ] {
                for inline in node.find_all(KindMatcher::new(inline_kind, lang.clone())) {
                    items.push(processors::inline_item(&inline, &owner, inline_kind));
                }
            }
        }
    }

    for (start, end, table_kind, rows, cols) in
        rst_helpers::detect_table_blocks(&root.root().text())
    {
        let owner = owner_path_for_line(&section_ctx, start);
        items.push(processors::virtual_table_item(
            start,
            end,
            &table_kind,
            rows,
            cols,
            &owner,
        ));
    }

    resolve_references(&mut items);

    Ok(items)
}

#[cfg(test)]
#[path = "../rst/tests/mod.rs"]
mod tests;
