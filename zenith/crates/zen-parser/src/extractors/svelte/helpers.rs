use ast_grep_core::Node;
use std::collections::HashSet;

pub(super) type Attr = (String, Option<String>);

pub(super) fn extract_tag_info<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<(String, Vec<Attr>)> {
    for child in node.children() {
        if matches!(child.kind().as_ref(), "start_tag" | "self_closing_tag") {
            let tag_name = child
                .children()
                .find(|c| c.kind().as_ref() == "tag_name")?
                .text()
                .to_string();
            let attrs = extract_attrs(&child);
            return Some((tag_name, attrs));
        }
    }
    None
}

pub(super) fn extract_tag_attrs<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<Attr> {
    for child in node.children() {
        if matches!(child.kind().as_ref(), "start_tag" | "self_closing_tag") {
            return extract_attrs(&child);
        }
    }
    Vec::new()
}

fn extract_attrs<D: ast_grep_core::Doc>(tag_node: &Node<D>) -> Vec<Attr> {
    tag_node
        .children()
        .filter(|c| c.kind().as_ref() == "attribute")
        .filter_map(|attr| {
            let name = attr
                .children()
                .find(|c| c.kind().as_ref() == "attribute_name")?
                .text()
                .to_string();

            let value = attr
                .children()
                .find(|c| {
                    matches!(
                        c.kind().as_ref(),
                        "quoted_attribute_value" | "attribute_value" | "expression"
                    )
                })
                .map(|v| v.text().to_string());

            Some((name, value))
        })
        .collect()
}

pub(super) fn attr_value(attrs: &[Attr], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(n, _)| n == name)
        .and_then(|(_, v)| v.clone())
}

pub(super) fn is_component_tag(tag: &str) -> bool {
    tag.chars().next().is_some_and(|c| c.is_ascii_uppercase()) || tag.contains('-')
}

pub(super) fn signature(tag: &str, attrs: &[Attr]) -> String {
    use std::fmt::Write;
    let mut s = format!("<{tag}");
    for (name, value) in attrs {
        if let Some(v) = value {
            let _ = write!(s, " {name}=\"{v}\"");
        } else {
            let _ = write!(s, " {name}");
        }
    }
    s.push('>');
    s
}

pub(super) fn normalize_attr_name(name: &str) -> String {
    name.trim().trim_matches('"').trim_matches('\'').to_string()
}

pub(super) fn directive_prefix(name: &str) -> Option<&'static str> {
    let n = normalize_attr_name(name);
    if n.starts_with("on:") {
        Some("on")
    } else if n.starts_with("bind:") {
        Some("bind")
    } else if n.starts_with("class:") {
        Some("class")
    } else if n.starts_with("use:") {
        Some("use")
    } else if n.starts_with("transition:") {
        Some("transition")
    } else if n.starts_with("in:") {
        Some("in")
    } else if n.starts_with("out:") {
        Some("out")
    } else if n.starts_with("animate:") {
        Some("animate")
    } else if n.starts_with("let:") {
        Some("let")
    } else {
        None
    }
}

pub(super) fn directive_name(name: &str) -> Option<String> {
    let n = normalize_attr_name(name);
    n.split_once(':').map(|(_, rhs)| rhs.to_string())
}

pub(super) fn unique_ids_from_attrs(attrs: &[Attr]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (name, val) in attrs {
        if normalize_attr_name(name) != "id" {
            continue;
        }
        if let Some(v) = val {
            let id = v.trim_matches('"').to_string();
            if seen.insert(id.clone()) {
                out.push(id);
            }
        }
    }
    out
}

pub(super) fn first_non_empty_line(text: &str) -> String {
    text.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .to_string()
}
