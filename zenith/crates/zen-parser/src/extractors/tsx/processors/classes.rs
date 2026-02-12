use ast_grep_core::Node;

use crate::types::{ParsedItem, SymbolKind, TsxMetadataExt};

use super::super::tsx_helpers::collect_jsx_tags_recursive;
use super::ClassInfo;

// ── Class component enrichment ─────────────────────────────────────

pub fn enrich_class_item(item: &mut ParsedItem, class_infos: &[ClassInfo]) {
    let info = class_infos
        .iter()
        .find(|c| c.start_line == item.start_line && c.name == item.name);

    let Some(ci) = info else {
        return;
    };

    let is_class_component = ci.extends_react_component || ci.extends_pure_component;
    let is_error_boundary = ci.has_derived_state_from_error || ci.has_component_did_catch;

    item.metadata.set_class_component(is_class_component);
    item.metadata.set_error_boundary(is_error_boundary);

    if is_class_component {
        item.metadata.set_component(true);
        item.kind = SymbolKind::Component;
    }

    if !ci.jsx_elements.is_empty() {
        item.metadata.set_jsx_elements(ci.jsx_elements.clone());
    }
    item.metadata.set_props_type_if_none(ci.props_type.clone());
}

// ── Class component collection ─────────────────────────────────────

pub fn collect_class_components<D: ast_grep_core::Doc>(node: &Node<D>, out: &mut Vec<ClassInfo>) {
    let kind = node.kind();
    match kind.as_ref() {
        "export_statement" => {
            let children: Vec<_> = node.children().collect();
            for child in &children {
                if child.kind().as_ref() == "class_declaration"
                    && let Some(ci) = analyze_class(child, node)
                {
                    out.push(ci);
                }
            }
        }
        "class_declaration" => {
            if let Some(ci) = analyze_class(node, node) {
                out.push(ci);
            }
        }
        _ => {}
    }

    let children: Vec<_> = node.children().collect();
    for child in &children {
        collect_class_components(child, out);
    }
}

fn analyze_class<D: ast_grep_core::Doc>(node: &Node<D>, anchor: &Node<D>) -> Option<ClassInfo> {
    let name = node.field("name").map(|n| n.text().to_string())?;

    // Check extends clause for React.Component / React.PureComponent.
    let (extends_component, extends_pure, props_type) = check_class_heritage(node);

    if !extends_component && !extends_pure {
        return None;
    }

    // Check class body for render(), getDerivedStateFromError, componentDidCatch.
    let body = node.field("body")?;
    let body_children: Vec<_> = body.children().collect();

    let mut has_derived_state = false;
    let mut has_did_catch = false;
    let mut jsx_elems = Vec::new();

    for member in &body_children {
        let mk = member.kind();
        if mk.as_ref() == "method_definition" {
            let method_name = member.field("name").map(|n| n.text().to_string());
            match method_name.as_deref() {
                Some("render") => {
                    if let Some(mbody) = member.field("body") {
                        collect_jsx_tags_recursive(&mbody, &mut jsx_elems);
                    }
                }
                Some("getDerivedStateFromError") => has_derived_state = true,
                Some("componentDidCatch") => has_did_catch = true,
                _ => {}
            }
        }
    }

    jsx_elems.sort();
    jsx_elems.dedup();

    Some(ClassInfo {
        start_line: anchor.start_pos().line() as u32 + 1,
        name,
        extends_react_component: extends_component,
        extends_pure_component: extends_pure,
        has_derived_state_from_error: has_derived_state,
        has_component_did_catch: has_did_catch,
        jsx_elements: jsx_elems,
        props_type,
    })
}

/// Check if a class extends `React.Component` or `React.PureComponent`.
/// Also extracts the props type parameter if present.
fn check_class_heritage<D: ast_grep_core::Doc>(node: &Node<D>) -> (bool, bool, Option<String>) {
    let children: Vec<_> = node.children().collect();
    for child in &children {
        if child.kind().as_ref() == "class_heritage" {
            let clauses: Vec<_> = child.children().collect();
            for clause in &clauses {
                if clause.kind().as_ref() == "extends_clause" {
                    let clause_children: Vec<_> = clause.children().collect();
                    for cc in &clause_children {
                        let ck = cc.kind();
                        if ck.as_ref() == "member_expression" {
                            let text = cc.text().to_string();
                            let is_component = text == "React.Component" || text == "Component";
                            let is_pure = text == "React.PureComponent" || text == "PureComponent";

                            // Extract props type from type_arguments: <Props, State>
                            let props = clause_children
                                .iter()
                                .find(|c| c.kind().as_ref() == "type_arguments")
                                .and_then(|ta| {
                                    ta.children()
                                        .find(|c| {
                                            let k = c.kind();
                                            k.as_ref() != "<"
                                                && k.as_ref() != ">"
                                                && k.as_ref() != ","
                                        })
                                        .map(|first| first.text().to_string())
                                });

                            return (is_component, is_pure, props);
                        }
                    }
                }
            }
        }
    }
    (false, false, None)
}
