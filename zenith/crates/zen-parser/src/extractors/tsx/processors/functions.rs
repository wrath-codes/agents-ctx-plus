use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, TsxMetadataExt};

use super::super::tsx_helpers::{
    collect_hooks_recursive, collect_jsx_tags_recursive, extract_props_from_arrow_params,
    extract_props_from_type_annotation, extract_props_type_from_params, has_jsx_recursive,
    is_component_name, is_component_return_type, is_hoc_name, is_hook_name,
};
use super::FnBody;

// ── Function/arrow enrichment ──────────────────────────────────────

pub fn enrich_fn_item(item: &mut ParsedItem, bodies: &[FnBody]) {
    let name = &item.name;

    let body = bodies
        .iter()
        .find(|b| b.start_line == item.start_line && b.name == *name);

    let is_hook = is_hook_name(name);
    let is_hoc = is_hoc_name(name);
    let is_forward_ref = body.is_some_and(|b| b.is_forward_ref);
    let is_memo = body.is_some_and(|b| b.is_memo);
    let is_lazy = body.is_some_and(|b| b.is_lazy);

    // A function is a component if: uppercase name AND (body has JSX OR
    // return type / type annotation indicates React component).
    let is_component = !is_hoc
        && !is_lazy
        && (is_forward_ref
            || is_memo
            || (is_component_name(name)
                && (body.is_some_and(|b| b.has_jsx)
                    || is_component_return_type(item.metadata.return_type.as_deref())
                    || body
                        .is_some_and(|b| is_component_return_type(b.type_annotation.as_deref())))));

    item.metadata.set_component(is_component);
    item.metadata.set_hook(is_hook);
    item.metadata.set_hoc(is_hoc);
    item.metadata.set_forward_ref(is_forward_ref);
    item.metadata.set_memo(is_memo);
    item.metadata.set_lazy(is_lazy);

    if is_component {
        item.kind = SymbolKind::Component;
    }

    if let Some(b) = body {
        if !b.hooks_used.is_empty() {
            item.metadata.set_hooks_used(b.hooks_used.clone());
        }
        if !b.jsx_elements.is_empty() {
            item.metadata.set_jsx_elements(b.jsx_elements.clone());
        }
        item.metadata.set_props_type_if_none(b.props_type.clone());
    }
}

// ── AST body collection (functions/arrows) ─────────────────────────

/// Walk the AST collecting function/arrow bodies with their metadata.
pub fn collect_fn_bodies<D: ast_grep_core::Doc>(node: &Node<D>, out: &mut Vec<FnBody>) {
    let kind = node.kind();
    match kind.as_ref() {
        "export_statement" => {
            let children: Vec<_> = node.children().collect();
            for child in &children {
                let ck = child.kind();
                match ck.as_ref() {
                    "function_declaration" => {
                        if let Some(fb) = analyze_function(child, node) {
                            out.push(fb);
                        }
                    }
                    "lexical_declaration" => {
                        collect_from_lexical(child, node, out);
                    }
                    _ => {}
                }
            }
        }
        "function_declaration" => {
            if let Some(fb) = analyze_function(node, node) {
                out.push(fb);
            }
        }
        "lexical_declaration" => {
            collect_from_lexical(node, node, out);
        }
        _ => {}
    }

    let children: Vec<_> = node.children().collect();
    for child in &children {
        collect_fn_bodies(child, out);
    }
}

fn collect_from_lexical<D: ast_grep_core::Doc>(
    node: &Node<D>,
    anchor: &Node<D>,
    out: &mut Vec<FnBody>,
) {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "variable_declarator"
            && let Some(fb) = analyze_variable_declarator(child, anchor)
        {
            out.push(fb);
        }
    }
}

/// Analyze a `function_declaration` node.
fn analyze_function<D: ast_grep_core::Doc>(node: &Node<D>, anchor: &Node<D>) -> Option<FnBody> {
    let name = node.field("name").map(|n| n.text().to_string())?;
    let body = node.field("body")?;

    let has_jsx = has_jsx_recursive(&body);
    let mut hooks = Vec::new();
    collect_hooks_recursive(&body, &mut hooks);
    hooks.sort();
    hooks.dedup();

    let mut jsx_elems = Vec::new();
    collect_jsx_tags_recursive(&body, &mut jsx_elems);
    jsx_elems.sort();
    jsx_elems.dedup();

    let props_type = extract_props_type_from_params(node);

    Some(FnBody {
        start_line: anchor.start_pos().line() as u32 + 1,
        name,
        has_jsx,
        hooks_used: hooks,
        jsx_elements: jsx_elems,
        is_forward_ref: false,
        is_memo: false,
        is_lazy: false,
        props_type,
        type_annotation: None,
    })
}

/// Analyze a `variable_declarator` node (arrow functions, `forwardRef`, `memo`, `lazy`).
fn analyze_variable_declarator<D: ast_grep_core::Doc>(
    declarator: &Node<D>,
    anchor: &Node<D>,
) -> Option<FnBody> {
    let name = declarator.field("name").map(|n| n.text().to_string())?;
    let value = declarator.field("value")?;

    let value_kind = value.kind();
    let vk = value_kind.as_ref();

    let type_annotation = declarator
        .children()
        .find(|c| c.kind().as_ref() == "type_annotation")
        .map(|ta| {
            ta.text()
                .to_string()
                .trim_start_matches(':')
                .trim()
                .to_string()
        });

    if vk == "arrow_function" {
        let (has_jsx, hooks, jsx_elems) = analyze_node_content(&value);
        let props_type = extract_props_from_type_annotation(type_annotation.as_deref())
            .or_else(|| extract_props_from_arrow_params(&value));

        Some(FnBody {
            start_line: anchor.start_pos().line() as u32 + 1,
            name,
            has_jsx,
            hooks_used: hooks,
            jsx_elements: jsx_elems,
            is_forward_ref: false,
            is_memo: false,
            is_lazy: false,
            props_type,
            type_annotation,
        })
    } else if vk == "call_expression" {
        let callee_name = extract_callee_name(&value);
        let is_fwd = matches!(
            callee_name.as_deref(),
            Some("React.forwardRef" | "forwardRef")
        );
        let is_memo = matches!(callee_name.as_deref(), Some("React.memo" | "memo"));
        let is_lazy = matches!(callee_name.as_deref(), Some("React.lazy" | "lazy"));

        let (has_jsx, hooks, jsx_elems) = if is_lazy {
            (false, Vec::new(), Vec::new())
        } else {
            find_and_analyze_inner_fn(&value)
        };

        let props_type = extract_props_from_type_annotation(type_annotation.as_deref());

        Some(FnBody {
            start_line: anchor.start_pos().line() as u32 + 1,
            name,
            has_jsx,
            hooks_used: hooks,
            jsx_elements: jsx_elems,
            is_forward_ref: is_fwd,
            is_memo,
            is_lazy,
            props_type,
            type_annotation,
        })
    } else {
        None
    }
}

// ── Body content analysis ──────────────────────────────────────────

/// Analyze a node (arrow function or `statement_block`) for JSX/hooks content.
fn analyze_node_content<D: ast_grep_core::Doc>(node: &Node<D>) -> (bool, Vec<String>, Vec<String>) {
    let target = node.field("body");
    let scan = target.as_ref().unwrap_or(node);
    let has = has_jsx_recursive(scan);
    let mut h = Vec::new();
    collect_hooks_recursive(scan, &mut h);
    h.sort();
    h.dedup();
    let mut j = Vec::new();
    collect_jsx_tags_recursive(scan, &mut j);
    j.sort();
    j.dedup();
    (has, h, j)
}

/// Find an arrow function or function expression inside a `call_expression`
/// and analyze its content in-place.
fn find_and_analyze_inner_fn<D: ast_grep_core::Doc>(
    call: &Node<D>,
) -> (bool, Vec<String>, Vec<String>) {
    let children: Vec<_> = call.children().collect();
    for child in &children {
        if child.kind().as_ref() == "arguments" {
            let args: Vec<_> = child.children().collect();
            for arg in &args {
                let ak = arg.kind();
                if ak.as_ref() == "arrow_function" || ak.as_ref() == "function_expression" {
                    return analyze_node_content(arg);
                }
            }
        }
    }
    (false, Vec::new(), Vec::new())
}

/// Extract the callee name from a `call_expression`.
fn extract_callee_name<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<String> {
    let children: Vec<_> = node.children().collect();
    children.first().map(|c| c.text().to_string())
}
