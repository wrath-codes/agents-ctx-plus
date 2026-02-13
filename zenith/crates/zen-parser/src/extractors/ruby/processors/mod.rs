use std::collections::HashSet;

use ast_grep_core::Node;

use crate::types::{CommonMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::ruby_helpers;

pub(super) fn process_type_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    let kind = match node.kind().as_ref() {
        "class" => SymbolKind::Class,
        "module" => SymbolKind::Module,
        _ => return None,
    };

    let local_name = node
        .field("name")
        .map(|name| ruby_helpers::extract_const_path(&name))?;
    let full_name = qualify_name(node, &local_name);
    let mut metadata = SymbolMetadata::default();

    if kind == SymbolKind::Class
        && let Some(role) = infer_rails_role(node, &full_name)
    {
        metadata.push_attribute(format!("rails:kind:{role}"));
    }

    Some(build_item(
        node,
        kind,
        full_name,
        Visibility::Public,
        metadata,
    ))
}

pub(super) fn process_method_declaration<D: ast_grep_core::Doc>(
    node: &Node<D>,
) -> Option<ParsedItem> {
    let method_name = ruby_helpers::extract_method_name(node)?;
    let owner = owner_context(node);
    let static_member = is_static_method(node);

    let kind = if method_name == "initialize"
        && owner.as_ref().is_some_and(|o| o.kind == SymbolKind::Class)
    {
        SymbolKind::Constructor
    } else if owner.is_some() {
        SymbolKind::Method
    } else {
        SymbolKind::Function
    };

    let visibility = resolve_method_visibility(node, &method_name, static_member);
    let mut metadata = SymbolMetadata::default();
    metadata.set_parameters(ruby_helpers::extract_method_parameters(node));

    if let Some(owner) = owner {
        metadata.set_owner_name(Some(owner.name));
        metadata.set_owner_kind(Some(owner.kind));
    }

    if static_member {
        metadata.mark_static_member();
    }

    if is_inside_concern_block(node, "included") {
        metadata.push_attribute("rails:concern:included");
    }
    if is_inside_concern_block(node, "class_methods") {
        metadata.push_attribute("rails:concern:class_methods");
    }

    if static_member && visibility == Visibility::Private {
        metadata.push_attribute("ruby:private_class_method");
    }

    Some(build_item(node, kind, method_name, visibility, metadata))
}

pub(super) fn process_assignment<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<ParsedItem> {
    let left = node.field("left")?;
    let owner = owner_context(node);
    let mut metadata = SymbolMetadata::default();

    if let Some(owner) = owner {
        metadata.set_owner_name(Some(owner.name));
        metadata.set_owner_kind(Some(owner.kind));
    }

    if is_inside_concern_block(node, "included") {
        metadata.push_attribute("rails:concern:included");
    }
    if is_inside_concern_block(node, "class_methods") {
        metadata.push_attribute("rails:concern:class_methods");
    }

    match left.kind().as_ref() {
        "constant" | "scope_resolution" => Some(build_item(
            node,
            SymbolKind::Const,
            ruby_helpers::extract_const_path(&left),
            Visibility::Public,
            metadata,
        )),
        "instance_variable" | "class_variable" => Some(build_item(
            node,
            SymbolKind::Field,
            left.text().to_string(),
            Visibility::Private,
            metadata,
        )),
        _ => None,
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn process_call<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<ParsedItem> {
    let Some(call_name) = ruby_helpers::call_method_name(node) else {
        return Vec::new();
    };

    if ruby_helpers::map_visibility(&call_name).is_some() || call_name == "private_class_method" {
        return Vec::new();
    }

    let Some(owner) = owner_context(node) else {
        return Vec::new();
    };

    let static_member = is_inside_concern_block(node, "class_methods");
    let concern_included = is_inside_concern_block(node, "included");
    let concern_class_methods = static_member;

    let mut items = Vec::new();
    let arg_texts = ruby_helpers::call_argument_texts(node);
    let symbol_args: Vec<String> = arg_texts
        .iter()
        .filter_map(|arg| ruby_helpers::extract_symbol_like_name(arg))
        .collect();

    match call_name.as_str() {
        "attr_reader" | "attr_writer" | "attr_accessor" => {
            for prop in symbol_args {
                let mut metadata = member_metadata(&owner, static_member);
                metadata.push_attribute(format!("ruby:{call_name}"));
                if concern_included {
                    metadata.push_attribute("rails:concern:included");
                }
                if concern_class_methods {
                    metadata.push_attribute("rails:concern:class_methods");
                }
                items.push(build_item(
                    node,
                    SymbolKind::Property,
                    prop,
                    Visibility::Public,
                    metadata,
                ));
            }
        }
        "belongs_to" | "has_many" | "has_one" | "has_and_belongs_to_many" => {
            for relation in symbol_args {
                let mut metadata = member_metadata(&owner, false);
                let tag = if call_name == "has_and_belongs_to_many" {
                    "rails:habtm".to_string()
                } else {
                    format!("rails:{call_name}")
                };
                metadata.push_attribute(tag);
                if concern_included {
                    metadata.push_attribute("rails:concern:included");
                }
                items.push(build_item(
                    node,
                    SymbolKind::Property,
                    relation,
                    Visibility::Public,
                    metadata,
                ));
            }
        }
        "scope" => {
            if let Some(scope_name) = symbol_args.first() {
                let mut metadata = member_metadata(&owner, true);
                metadata.push_attribute("rails:scope");
                if concern_included {
                    metadata.push_attribute("rails:concern:included");
                }
                items.push(build_item(
                    node,
                    SymbolKind::Method,
                    scope_name.clone(),
                    Visibility::Public,
                    metadata,
                ));
            }
        }
        "enum" => {
            if let Some(enum_name) = symbol_args.first() {
                let mut metadata = member_metadata(&owner, false);
                metadata.push_attribute("rails:enum");
                if concern_included {
                    metadata.push_attribute("rails:concern:included");
                }
                items.push(build_item(
                    node,
                    SymbolKind::Property,
                    enum_name.clone(),
                    Visibility::Public,
                    metadata,
                ));
            }
        }
        "delegate" => {
            for delegated in symbol_args {
                let mut metadata = member_metadata(&owner, false);
                metadata.push_attribute("rails:delegate");
                if concern_included {
                    metadata.push_attribute("rails:concern:included");
                }
                items.push(build_item(
                    node,
                    SymbolKind::Method,
                    delegated,
                    Visibility::Public,
                    metadata,
                ));
            }
        }
        name if is_rails_callback(name)
            || matches!(
                name,
                "validates" | "validate" | "before_action" | "helper_method"
            ) =>
        {
            let mut metadata = member_metadata(&owner, static_member);
            metadata.push_attribute(format!("rails:{name}"));
            metadata.set_parameters(arg_texts);
            if concern_included {
                metadata.push_attribute("rails:concern:included");
            }
            if concern_class_methods {
                metadata.push_attribute("rails:concern:class_methods");
            }
            items.push(build_item(
                node,
                SymbolKind::Module,
                name.to_string(),
                Visibility::Public,
                metadata,
            ));
        }
        _ => {}
    }

    items
}

pub(super) fn dedupe(items: Vec<ParsedItem>) -> Vec<ParsedItem> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for item in items {
        let key = format!(
            "{}:{}:{}:{}:{}",
            item.kind,
            item.name,
            item.start_line,
            item.metadata.owner_name.as_deref().unwrap_or_default(),
            item.metadata.attributes.join("|")
        );
        if seen.insert(key) {
            out.push(item);
        }
    }

    out
}

#[derive(Clone)]
struct OwnerContext {
    name: String,
    kind: SymbolKind,
}

fn member_metadata(owner: &OwnerContext, static_member: bool) -> SymbolMetadata {
    let mut metadata = SymbolMetadata::default();
    metadata.set_owner_name(Some(owner.name.clone()));
    metadata.set_owner_kind(Some(owner.kind));
    if static_member {
        metadata.mark_static_member();
    }
    metadata
}

fn owner_context<D: ast_grep_core::Doc>(node: &Node<D>) -> Option<OwnerContext> {
    let chain = owner_chain(node, false);
    chain.last().cloned()
}

fn owner_chain<D: ast_grep_core::Doc>(node: &Node<D>, include_self: bool) -> Vec<OwnerContext> {
    let mut segments = Vec::new();

    if include_self
        && matches!(node.kind().as_ref(), "class" | "module")
        && let Some(name_node) = node.field("name")
    {
        segments.push((
            ruby_helpers::extract_const_path(&name_node),
            if node.kind().as_ref() == "class" {
                SymbolKind::Class
            } else {
                SymbolKind::Module
            },
        ));
    }

    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(parent.kind().as_ref(), "class" | "module")
            && let Some(name_node) = parent.field("name")
        {
            let owner_kind = if parent.kind().as_ref() == "class" {
                SymbolKind::Class
            } else {
                SymbolKind::Module
            };
            segments.push((ruby_helpers::extract_const_path(&name_node), owner_kind));
        }
        current = parent.parent();
    }

    segments.reverse();

    let mut full_chain = Vec::new();
    let mut current_path = String::new();
    for (segment, kind) in segments {
        if segment.contains("::") {
            current_path = ruby_helpers::normalize_const_path(&segment);
        } else if current_path.is_empty() {
            current_path = segment;
        } else {
            current_path = format!("{current_path}::{segment}");
        }

        full_chain.push(OwnerContext {
            name: current_path.clone(),
            kind,
        });
    }

    full_chain
}

fn qualify_name<D: ast_grep_core::Doc>(node: &Node<D>, local_name: &str) -> String {
    if local_name.contains("::") {
        return ruby_helpers::normalize_const_path(local_name);
    }

    let owner_chain = owner_chain(node, false);
    if let Some(owner) = owner_chain.last() {
        return format!("{}::{local_name}", owner.name);
    }

    local_name.to_string()
}

fn is_static_method<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    if node.kind().as_ref() == "singleton_method" {
        return true;
    }
    is_inside_concern_block(node, "class_methods")
}

fn is_inside_concern_block<D: ast_grep_core::Doc>(node: &Node<D>, call_name: &str) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind().as_ref() == "call"
            && ruby_helpers::call_method_name(&parent).as_deref() == Some(call_name)
        {
            return true;
        }
        current = parent.parent();
    }
    false
}

fn resolve_method_visibility<D: ast_grep_core::Doc>(
    node: &Node<D>,
    method_name: &str,
    static_member: bool,
) -> Visibility {
    let text = node.text().trim_start().to_string();
    if text.starts_with("private def") {
        return Visibility::Private;
    }
    if text.starts_with("protected def") {
        return Visibility::Protected;
    }
    if text.starts_with("public def") {
        return Visibility::Public;
    }

    if let Some(visibility) = explicit_visibility_override(node, method_name, static_member) {
        return visibility;
    }

    let mut section_visibility = None;
    let mut current = node.prev();
    while let Some(sibling) = current {
        let kind = sibling.kind();
        let kr = kind.as_ref();
        if kr == "comment" {
            current = sibling.prev();
            continue;
        }

        if kr == "call" {
            let call_name = ruby_helpers::call_method_name(&sibling);
            let receiver = ruby_helpers::call_receiver_text(&sibling);
            if let Some(call_name) = call_name
                && receiver.is_none()
            {
                let args = ruby_helpers::call_argument_texts(&sibling);
                let symbols: Vec<String> = args
                    .iter()
                    .filter_map(|arg| ruby_helpers::extract_symbol_like_name(arg))
                    .collect();

                if let Some(visibility) = ruby_helpers::map_visibility(&call_name) {
                    if symbols.iter().any(|symbol| symbol == method_name) {
                        return visibility;
                    }
                    if symbols.is_empty() && section_visibility.is_none() {
                        section_visibility = Some(visibility);
                    }
                }
            }
        }

        let sibling_text = sibling.text().trim().to_string();
        if let Some(visibility) = ruby_helpers::map_visibility(&sibling_text)
            && section_visibility.is_none()
        {
            section_visibility = Some(visibility);
        }

        for (directive, visibility) in [
            ("private", Visibility::Private),
            ("protected", Visibility::Protected),
            ("public", Visibility::Public),
        ] {
            if sibling_text.starts_with(&format!("{directive} ")) {
                let symbols = symbol_targets_from_directive(&sibling_text);
                if symbols.iter().any(|symbol| symbol == method_name) {
                    return visibility;
                }
            }
        }

        if static_member
            && sibling_text.starts_with("private_class_method")
            && symbol_targets_from_directive(&sibling_text)
                .iter()
                .any(|symbol| symbol == method_name)
        {
            return Visibility::Private;
        }

        current = sibling.prev();
    }

    if static_member {
        Visibility::Public
    } else {
        section_visibility.unwrap_or(Visibility::Public)
    }
}

fn symbol_targets_from_directive(text: &str) -> Vec<String> {
    text.split(|ch: char| ch.is_whitespace() || ch == ',' || ch == '(' || ch == ')')
        .filter_map(|part| part.strip_prefix(':'))
        .map(|symbol| {
            symbol
                .trim_matches('"')
                .trim_matches('\'')
                .trim_matches(':')
                .to_string()
        })
        .filter(|symbol| !symbol.is_empty())
        .collect()
}

fn explicit_visibility_override<D: ast_grep_core::Doc>(
    node: &Node<D>,
    method_name: &str,
    static_member: bool,
) -> Option<Visibility> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(parent.kind().as_ref(), "class" | "module") {
            for line in parent.text().lines() {
                let trimmed = line.trim();
                if static_member && trimmed.starts_with("private_class_method") {
                    let symbols = symbol_targets_from_directive(trimmed);
                    if symbols.iter().any(|symbol| symbol == method_name) {
                        return Some(Visibility::Private);
                    }
                }

                for (directive, visibility) in [
                    ("private", Visibility::Private),
                    ("protected", Visibility::Protected),
                    ("public", Visibility::Public),
                ] {
                    if trimmed.starts_with(&format!("{directive} ")) {
                        let symbols = symbol_targets_from_directive(trimmed);
                        if symbols.iter().any(|symbol| symbol == method_name) {
                            return Some(visibility);
                        }
                    }
                }
            }
            return None;
        }
        current = parent.parent();
    }

    None
}

fn infer_rails_role<D: ast_grep_core::Doc>(node: &Node<D>, name: &str) -> Option<&'static str> {
    let superclass = node.field("superclass").map(|superclass| {
        superclass
            .text()
            .trim()
            .trim_start_matches('<')
            .trim()
            .to_string()
    });

    if let Some(superclass) = superclass {
        if superclass.ends_with("ApplicationRecord") {
            return Some("model");
        }
        if superclass.ends_with("ApplicationController") {
            return Some("controller");
        }
        if superclass.ends_with("ApplicationJob") {
            return Some("job");
        }
        if superclass.ends_with("ApplicationMailer") {
            return Some("mailer");
        }
        if superclass.ends_with("ApplicationCable::Channel") {
            return Some("channel");
        }
    }

    if name.ends_with("Controller") {
        return Some("controller");
    }
    if name.ends_with("Job") {
        return Some("job");
    }
    if name.ends_with("Mailer") {
        return Some("mailer");
    }
    if name.ends_with("Channel") {
        return Some("channel");
    }

    None
}

fn is_rails_callback(name: &str) -> bool {
    name.starts_with("before_") || name.starts_with("after_") || name.starts_with("around_")
}

fn build_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: SymbolKind,
    name: String,
    visibility: Visibility,
    metadata: SymbolMetadata,
) -> ParsedItem {
    ParsedItem {
        kind,
        name,
        signature: ruby_helpers::extract_ruby_signature(node),
        source: crate::extractors::helpers::extract_source(node, 40),
        doc_comment: ruby_helpers::extract_ruby_doc(node),
        start_line: node.start_pos().line() as u32 + 1,
        end_line: node.end_pos().line() as u32 + 1,
        visibility,
        metadata,
    }
}
