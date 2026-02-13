use ast_grep_core::Node;
use std::collections::{HashMap, HashSet};

use crate::extractors::helpers::extract_source;
use crate::types::{
    CommonMetadataExt, HtmlMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility,
};

use super::svelte_helpers;

fn build_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: SymbolKind,
    name: String,
    signature: String,
    mut metadata: SymbolMetadata,
) -> ParsedItem {
    metadata.push_attribute("svelte:extractor");
    ParsedItem {
        kind,
        name,
        signature,
        source: extract_source(node, 30),
        doc_comment: String::new(),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility: Visibility::Public,
        metadata,
    }
}

pub(super) fn root_item<D: ast_grep_core::Doc>(root: &Node<D>) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("svelte:kind:document");
    build_item(
        root,
        SymbolKind::Module,
        "$".to_string(),
        "svelte-document".to_string(),
        metadata,
    )
}

pub(super) fn script_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let attrs = svelte_helpers::extract_tag_attrs(node);
    let lang = svelte_helpers::attr_value(&attrs, "lang")
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_else(|| "js".to_string());
    let context = svelte_helpers::attr_value(&attrs, "context")
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_default();

    let script_text = node.text().to_string();

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("svelte:kind:script");
    metadata.push_attribute(format!("svelte:script_lang:{lang}"));
    if !context.is_empty() {
        metadata.push_attribute(format!("svelte:script_context:{context}"));
    }
    if matches!(lang.as_str(), "ts" | "typescript") {
        metadata.push_attribute("svelte:embedded_parser:typescript");
    } else {
        metadata.push_attribute("svelte:embedded_parser:javascript");
    }

    if script_text.contains("from '$app/") || script_text.contains("from \"$app/") {
        metadata.push_attribute("sveltekit:uses_app_modules");
    }
    for flag in ["prerender", "csr", "ssr", "trailingSlash", "load"] {
        if script_text.contains(&format!("export const {flag}"))
            || script_text.contains(&format!("export async function {flag}"))
            || script_text.contains(&format!("export function {flag}"))
        {
            metadata.push_attribute(format!("sveltekit:export:{flag}"));
        }
    }
    if script_text.contains("$props(") {
        metadata.push_attribute("svelte:uses_props_api");
    }

    build_item(
        node,
        SymbolKind::Module,
        if context == "module" {
            "script:module".to_string()
        } else {
            "script:instance".to_string()
        },
        svelte_helpers::signature("script", &attrs),
        metadata,
    )
}

pub(super) fn style_item<D: ast_grep_core::Doc>(node: &Node<D>) -> ParsedItem {
    let attrs = svelte_helpers::extract_tag_attrs(node);
    let lang = svelte_helpers::attr_value(&attrs, "lang")
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_else(|| "css".to_string());

    let text = node.text().to_string();
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("svelte:kind:style");
    metadata.push_attribute(format!("svelte:style_lang:{lang}"));
    metadata.push_attribute("svelte:embedded_parser:css");
    if text.contains(":global(") || text.contains(":global ") {
        metadata.push_attribute("svelte:style_global_selector");
    }
    let var_count = text.matches("--").count();
    if var_count > 0 {
        metadata.push_attribute(format!("svelte:style_css_vars:{var_count}"));
    }

    build_item(
        node,
        SymbolKind::Module,
        "style".to_string(),
        svelte_helpers::signature("style", &attrs),
        metadata,
    )
}

pub(super) fn element_item<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let (tag, attrs) = svelte_helpers::extract_tag_info(node)?;
    let is_component = svelte_helpers::is_component_tag(&tag);

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("svelte:kind:element");
    metadata.push_attribute("svelte:embedded_parser:html");
    metadata.set_tag_name(tag.clone());
    metadata.set_html_attributes(attrs.clone());

    let mut directive_count = 0usize;
    let mut event_count = 0usize;
    for (name, _) in &attrs {
        if let Some(prefix) = svelte_helpers::directive_prefix(name) {
            directive_count += 1;
            if prefix == "on" {
                event_count += 1;
            }
        }
    }
    if directive_count > 0 {
        metadata.push_attribute(format!("svelte:directives:{directive_count}"));
    }
    if event_count > 0 {
        metadata.push_attribute(format!("svelte:events:{event_count}"));
    }

    if tag.starts_with("svelte:") {
        metadata.push_attribute("svelte:special_element");
    }

    let kind = if is_component {
        metadata.push_attribute("svelte:component");
        SymbolKind::Component
    } else {
        SymbolKind::Struct
    };

    let name = if is_component {
        tag.clone()
    } else {
        svelte_helpers::attr_value(&attrs, "id")
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_else(|| tag.clone())
    };

    Some(build_item(
        node,
        kind,
        name,
        svelte_helpers::signature(&tag, &attrs),
        metadata,
    ))
}

pub(super) fn directive_attr_items<D: ast_grep_core::Doc>(
    node: &Node<D>,
    owner: &str,
) -> Vec<ParsedItem> {
    let Some((_tag, attrs)) = svelte_helpers::extract_tag_info(node) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (name, value) in attrs {
        let Some(prefix) = svelte_helpers::directive_prefix(&name) else {
            continue;
        };
        let directive = svelte_helpers::directive_name(&name).unwrap_or_default();
        let mut metadata = SymbolMetadata::default();
        metadata.push_attribute("svelte:kind:directive_attribute");
        metadata.push_attribute(format!("svelte:directive_type:{prefix}"));
        metadata.push_attribute(format!("svelte:directive_name:{directive}"));
        if prefix == "on" {
            metadata.push_attribute("svelte:event_handler");
        }
        metadata.set_owner_name(Some(owner.to_string()));
        metadata.set_owner_kind(Some(SymbolKind::Struct));

        let signature = if let Some(v) = value {
            format!("{name}={}", v.trim())
        } else {
            name.clone()
        };
        out.push(build_item(
            node,
            SymbolKind::Property,
            format!("directive:{name}@{}", node.start_pos().line() + 1),
            signature,
            metadata,
        ));
    }
    out
}

pub(super) fn block_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    name: &str,
    kind_attr: &str,
) -> ParsedItem {
    let text = node.text().to_string();
    let first_line = svelte_helpers::first_non_empty_line(&text);
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(format!("svelte:kind:{kind_attr}"));

    match kind_attr {
        "if_statement" => {
            if let Some(cond) = extract_between(&first_line, "{#if", "}") {
                metadata.push_attribute(format!("svelte:if_condition:{}", cond.trim()));
            }
        }
        "each_statement" => {
            if let Some(parts) = extract_between(&first_line, "{#each", "}") {
                metadata.push_attribute(format!("svelte:each_expr:{}", parts.trim()));
            }
        }
        "await_statement" => {
            if let Some(expr) = extract_between(&first_line, "{#await", "}") {
                metadata.push_attribute(format!("svelte:await_expr:{}", expr.trim()));
            }
            if text.contains(":then") {
                metadata.push_attribute("svelte:await_has_then");
            }
            if text.contains(":catch") {
                metadata.push_attribute("svelte:await_has_catch");
            }
        }
        "key_statement" => {
            if let Some(expr) = extract_between(&first_line, "{#key", "}") {
                metadata.push_attribute(format!("svelte:key_expr:{}", expr.trim()));
            }
        }
        "snippet_statement" => {
            if let Some(sig) = extract_between(&first_line, "{#snippet", "}") {
                let s = sig.trim();
                metadata.push_attribute(format!("svelte:snippet_sig:{s}"));
                if let Some(name) = s.split('(').next() {
                    metadata.push_attribute(format!("svelte:snippet_name:{}", name.trim()));
                }
            }
        }
        _ => {}
    }

    build_item(
        node,
        SymbolKind::Property,
        format!("{name}-{}", node.start_pos().line() + 1),
        name.to_string(),
        metadata,
    )
}

pub(super) fn tag_item<D: ast_grep_core::Doc>(node: &Node<D>, name: &str) -> ParsedItem {
    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute(format!("svelte:kind:{name}"));
    if name == "render_tag" {
        if let Some(call) = extract_between(&node.text(), "{@render", "}") {
            metadata.push_attribute(format!("svelte:render_call:{}", call.trim()));
        }
    }
    build_item(
        node,
        SymbolKind::Property,
        format!("{name}-{}", node.start_pos().line() + 1),
        node.text().trim().to_string(),
        metadata,
    )
}

pub(super) fn script_api_items<D: ast_grep_core::Doc>(
    script_node: &Node<D>,
    owner: &str,
) -> Vec<ParsedItem> {
    let text = script_node.text().to_string();
    let mut out = Vec::new();

    for (kind, needle) in [
        ("prop", "export let "),
        ("export_const", "export const "),
        ("export_fn", "export function "),
        ("export_class", "export class "),
    ] {
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix(needle) {
                let name = rest
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                    .next()
                    .unwrap_or("");
                if name.is_empty() {
                    continue;
                }
                let mut metadata = SymbolMetadata::default();
                metadata.push_attribute("svelte:kind:script_api");
                metadata.push_attribute(format!("svelte:script_api_kind:{kind}"));
                metadata.set_owner_name(Some(owner.to_string()));
                metadata.set_owner_kind(Some(SymbolKind::Module));
                out.push(build_item(
                    script_node,
                    SymbolKind::Property,
                    format!("script_api:{name}"),
                    trimmed.to_string(),
                    metadata,
                ));
            }
        }
    }

    if text.contains("createEventDispatcher") {
        let mut metadata = SymbolMetadata::default();
        metadata.push_attribute("svelte:kind:event_dispatcher");
        metadata.set_owner_name(Some(owner.to_string()));
        metadata.set_owner_kind(Some(SymbolKind::Module));
        out.push(build_item(
            script_node,
            SymbolKind::Property,
            "script_api:createEventDispatcher".to_string(),
            "createEventDispatcher".to_string(),
            metadata,
        ));
    }

    let mut events = HashSet::new();
    for call in find_dispatch_calls(&text) {
        if events.insert(call.clone()) {
            let mut metadata = SymbolMetadata::default();
            metadata.push_attribute("svelte:kind:event_emit");
            metadata.push_attribute(format!("svelte:event:{call}"));
            metadata.set_owner_name(Some(owner.to_string()));
            metadata.set_owner_kind(Some(SymbolKind::Module));
            out.push(build_item(
                script_node,
                SymbolKind::Property,
                format!("event:{call}"),
                format!("dispatch('{call}')"),
                metadata,
            ));
        }
    }

    out
}

pub(super) fn duplicate_id_items<D: ast_grep_core::Doc>(root: &Node<D>) -> Vec<ParsedItem> {
    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut out = Vec::new();

    for node in root.find_all(ast_grep_core::matcher::KindMatcher::new(
        "element",
        root.lang().clone(),
    )) {
        let Some((_tag, attrs)) = svelte_helpers::extract_tag_info(&node) else {
            continue;
        };
        for id in svelte_helpers::unique_ids_from_attrs(&attrs) {
            let line = node.start_pos().line() as u32 + 1;
            if let Some(prev) = seen.insert(id.clone(), line) {
                let mut metadata = SymbolMetadata::default();
                metadata.push_attribute("svelte:kind:duplicate_id");
                metadata.push_attribute(format!("svelte:duplicate_id:{id}"));
                metadata.push_attribute(format!("svelte:duplicate_previous_line:{prev}"));
                out.push(build_item(
                    &node,
                    SymbolKind::Property,
                    format!("duplicate-id:{id}:{line}"),
                    format!("id={id}"),
                    metadata,
                ));
            }
        }
    }

    out
}

pub(super) fn link_snippet_render_refs(items: &mut [ParsedItem]) {
    let mut snippets: HashMap<String, String> = HashMap::new();
    for item in items.iter() {
        for attr in &item.metadata.attributes {
            if let Some(name) = attr.strip_prefix("svelte:snippet_name:") {
                snippets.insert(name.to_string(), item.name.clone());
            }
        }
    }

    for item in items.iter_mut() {
        let Some(call) = item
            .metadata
            .attributes
            .iter()
            .find_map(|a| a.strip_prefix("svelte:render_call:"))
        else {
            continue;
        };

        let callee = call
            .split('(')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        if callee.is_empty() {
            continue;
        }

        if let Some(target) = snippets.get(&callee) {
            item.metadata
                .attributes
                .push(format!("svelte:ref_target:{target}"));
        } else {
            item.metadata
                .attributes
                .push(format!("svelte:broken_snippet_ref:{callee}"));
        }
    }
}

fn extract_between<'a>(text: &'a str, left: &str, right: &str) -> Option<&'a str> {
    let start = text.find(left)? + left.len();
    let end = text[start..].find(right)? + start;
    Some(&text[start..end])
}

fn find_dispatch_calls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let needle = "dispatch(";
    while let Some(pos) = text[i..].find(needle) {
        let start = i + pos + needle.len();
        let tail = &text[start..];
        if let Some(stripped) = tail.strip_prefix('\'')
            && let Some(end) = stripped.find('\'')
        {
            out.push(stripped[..end].to_string());
            i = start + end + 2;
            continue;
        }
        if let Some(stripped) = tail.strip_prefix('"')
            && let Some(end) = stripped.find('"')
        {
            out.push(stripped[..end].to_string());
            i = start + end + 2;
            continue;
        }
        i = start;
    }
    out
}
