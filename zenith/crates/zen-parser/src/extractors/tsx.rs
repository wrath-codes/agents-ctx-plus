//! TSX/React rich extractor.
//!
//! Delegates to the [`typescript`](super::typescript) extractor for base
//! symbol extraction (functions, classes, interfaces, enums, etc.), then
//! enriches each item with React/JSX-specific metadata:
//!
//! - **Component detection**: uppercase name + JSX return / `React.FC` type
//! - **Hook detection**: `use*` naming convention
//! - **HOC detection**: `with*` naming convention + React return type
//! - **`forwardRef` detection**: `React.forwardRef(...)` call wrapper
//! - **`React.memo` detection**: `React.memo(...)` wrapper
//! - **`React.lazy` detection**: `React.lazy(() => import(...))` wrapper
//! - **Class component detection**: extends `React.Component` / `PureComponent`
//! - **Error boundary detection**: `getDerivedStateFromError` / `componentDidCatch`
//! - **Server directive detection**: `"use client"` / `"use server"` directives
//! - **Hooks-used**: which React hooks are called inside a function body
//! - **JSX elements**: which JSX tags are rendered
//! - **Props type**: the type annotation on the component's parameter

use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use crate::types::{ParsedItem, SymbolKind, TsxMetadataExt};

/// Extract all API symbols from a TSX source file with React metadata.
///
/// Runs the TypeScript extractor first, then walks the original AST a
/// second time to attach JSX/React information to each item.
///
/// # Errors
/// Returns `ParserError` if parsing fails.
pub fn extract<D: ast_grep_core::Doc<Lang = SupportLang>>(
    root: &ast_grep_core::AstGrep<D>,
    lang: SupportLang,
) -> Result<Vec<ParsedItem>, crate::error::ParserError> {
    let mut items = super::typescript::extract(root, lang)?;

    let root_node = root.root();

    // Detect file-level directive ("use client" / "use server").
    let directive = detect_directive(&root_node);

    // Build a lookup of function/arrow bodies for deeper analysis.
    let mut bodies: Vec<FnBody> = Vec::new();
    collect_fn_bodies(&root_node, &mut bodies);

    // Detect class components.
    let mut class_infos: Vec<ClassInfo> = Vec::new();
    collect_class_components(&root_node, &mut class_infos);

    for item in &mut items {
        enrich_item(item, &bodies, &class_infos);
        // Apply file-level directive to all items.
        if let Some(ref dir) = directive {
            item.metadata.set_component_directive(dir.clone());
        }
    }
    Ok(items)
}

// ── Enrichment pass ────────────────────────────────────────────────

/// Metadata extracted from a function/arrow body.
#[allow(clippy::struct_excessive_bools)]
struct FnBody {
    start_line: u32,
    name: String,
    has_jsx: bool,
    hooks_used: Vec<String>,
    jsx_elements: Vec<String>,
    is_forward_ref: bool,
    is_memo: bool,
    is_lazy: bool,
    props_type: Option<String>,
    type_annotation: Option<String>,
}

/// Metadata extracted from a class declaration.
#[allow(clippy::struct_excessive_bools)]
struct ClassInfo {
    start_line: u32,
    name: String,
    extends_react_component: bool,
    extends_pure_component: bool,
    has_derived_state_from_error: bool,
    has_component_did_catch: bool,
    jsx_elements: Vec<String>,
    props_type: Option<String>,
}

fn enrich_item(item: &mut ParsedItem, bodies: &[FnBody], class_infos: &[ClassInfo]) {
    match item.kind {
        SymbolKind::Function | SymbolKind::Const => enrich_fn_item(item, bodies),
        SymbolKind::Class => enrich_class_item(item, class_infos),
        _ => {}
    }
}

fn enrich_fn_item(item: &mut ParsedItem, bodies: &[FnBody]) {
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

fn enrich_class_item(item: &mut ParsedItem, class_infos: &[ClassInfo]) {
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

// ── Directive detection ────────────────────────────────────────────

/// Detect `"use client"` or `"use server"` directive at file top.
fn detect_directive<D: ast_grep_core::Doc>(root: &Node<D>) -> Option<String> {
    let children: Vec<_> = root.children().collect();
    for child in &children {
        let kind = child.kind();
        let k = kind.as_ref();
        if k == "expression_statement" {
            let inner: Vec<_> = child.children().collect();
            for ic in &inner {
                if ic.kind().as_ref() == "string" {
                    let text = ic.text().to_string();
                    let stripped = text.trim_matches(|c| c == '"' || c == '\'');
                    if stripped == "use client" || stripped == "use server" {
                        return Some(stripped.to_string());
                    }
                }
            }
        }
        // Stop after first non-comment, non-directive statement.
        if k != "comment" && k != "expression_statement" {
            break;
        }
    }
    None
}

// ── AST body collection (functions/arrows) ─────────────────────────

/// Walk the AST collecting function/arrow bodies with their metadata.
fn collect_fn_bodies<D: ast_grep_core::Doc>(node: &Node<D>, out: &mut Vec<FnBody>) {
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

// ── Class component collection ─────────────────────────────────────

fn collect_class_components<D: ast_grep_core::Doc>(node: &Node<D>, out: &mut Vec<ClassInfo>) {
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
/// (e.g., `React.memo((...) => ...)` or `React.memo(function X(...) { ... })`)
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

// ── JSX detection ──────────────────────────────────────────────────

fn has_jsx_recursive<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let kind = node.kind();
    let k = kind.as_ref();
    if k == "jsx_element" || k == "jsx_self_closing_element" || k == "jsx_fragment" {
        return true;
    }
    let children: Vec<_> = node.children().collect();
    children.iter().any(|c| has_jsx_recursive(c))
}

fn collect_jsx_tags_recursive<D: ast_grep_core::Doc>(node: &Node<D>, tags: &mut Vec<String>) {
    let kind = node.kind();
    let k = kind.as_ref();
    if k == "jsx_opening_element" || k == "jsx_self_closing_element" {
        let children: Vec<_> = node.children().collect();
        for child in &children {
            let ck = child.kind();
            if ck.as_ref() == "identifier" || ck.as_ref() == "member_expression" {
                let tag = child.text().to_string();
                if !tag.is_empty() {
                    tags.push(tag);
                }
                break;
            }
        }
    }
    let children: Vec<_> = node.children().collect();
    for child in &children {
        collect_jsx_tags_recursive(child, tags);
    }
}

// ── Hook detection ─────────────────────────────────────────────────

fn collect_hooks_recursive<D: ast_grep_core::Doc>(node: &Node<D>, hooks: &mut Vec<String>) {
    let kind = node.kind();
    if kind.as_ref() == "call_expression" {
        let children: Vec<_> = node.children().collect();
        if let Some(callee) = children.first() {
            let callee_text = callee.text().to_string();
            if is_hook_name(&callee_text) {
                hooks.push(callee_text);
            }
        }
    }
    let children: Vec<_> = node.children().collect();
    for child in &children {
        collect_hooks_recursive(child, hooks);
    }
}

// ── Props type extraction ──────────────────────────────────────────

/// Extract props type from function parameters.
///
/// Matches `({ ... }: PropsType)` in `formal_parameters`.
fn extract_props_type_from_params<D: ast_grep_core::Doc>(func: &Node<D>) -> Option<String> {
    let params = func.field("parameters")?;
    let children: Vec<_> = params.children().collect();
    for child in &children {
        if child.kind().as_ref() == "required_parameter" {
            let param_children: Vec<_> = child.children().collect();
            let has_object_pattern = param_children
                .iter()
                .any(|c| c.kind().as_ref() == "object_pattern");
            if has_object_pattern {
                for pc in &param_children {
                    if pc.kind().as_ref() == "type_annotation" {
                        return Some(
                            pc.text()
                                .to_string()
                                .trim_start_matches(':')
                                .trim()
                                .to_string(),
                        );
                    }
                }
            }
        }
    }
    None
}

/// Extract props type from a type annotation like `React.FC<UserCardProps>`.
fn extract_props_from_type_annotation(annotation: Option<&str>) -> Option<String> {
    let ann = annotation?;
    let start = ann.find('<')?;
    let end = ann.rfind('>')?;
    if end <= start {
        return None;
    }
    Some(ann[start + 1..end].to_string())
}

/// Extract props type from arrow function parameters.
fn extract_props_from_arrow_params<D: ast_grep_core::Doc>(arrow: &Node<D>) -> Option<String> {
    extract_props_type_from_params(arrow)
}

// ── Naming conventions ─────────────────────────────────────────────

/// React hook: starts with `use` followed by uppercase letter.
fn is_hook_name(name: &str) -> bool {
    name.starts_with("use")
        && name.len() > 3
        && name[3..4].chars().next().is_some_and(char::is_uppercase)
}

/// HOC: starts with `with` followed by uppercase letter.
fn is_hoc_name(name: &str) -> bool {
    name.starts_with("with")
        && name.len() > 4
        && name[4..5].chars().next().is_some_and(char::is_uppercase)
}

/// Component: starts with uppercase letter.
fn is_component_name(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// Check if a return type string indicates a React component.
fn is_component_return_type(rt: Option<&str>) -> bool {
    rt.is_some_and(|t| {
        t.contains("JSX.Element")
            || t.contains("ReactNode")
            || t.contains("ReactElement")
            || t.contains("React.FC")
            || t.contains("React.FunctionComponent")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SymbolKind, Visibility};
    use ast_grep_language::LanguageExt;
    use pretty_assertions::assert_eq;

    fn parse_and_extract(source: &str) -> Vec<ParsedItem> {
        let root = SupportLang::Tsx.ast_grep(source);
        extract(&root, SupportLang::Tsx).expect("extraction should succeed")
    }

    fn find_by_name<'a>(items: &'a [ParsedItem], name: &str) -> &'a ParsedItem {
        items.iter().find(|i| i.name == name).unwrap_or_else(|| {
            let names: Vec<_> = items
                .iter()
                .map(|i| format!("{}({})", i.name, i.kind))
                .collect();
            panic!("should find item named '{name}', available: {names:?}")
        })
    }

    // ── "use client" directive ─────────────────────────────────────

    #[test]
    fn use_client_directive_detected() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let btn = find_by_name(&items, "Button");
        assert_eq!(
            btn.metadata.component_directive.as_deref(),
            Some("use client")
        );
    }

    #[test]
    fn directive_applies_to_all_items() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        for item in &items {
            assert_eq!(
                item.metadata.component_directive.as_deref(),
                Some("use client"),
                "'{}' should have directive",
                item.name
            );
        }
    }

    #[test]
    fn use_server_directive() {
        let src = "\"use server\";\nexport async function submitForm() {}";
        let items = parse_and_extract(src);
        let f = find_by_name(&items, "submitForm");
        assert_eq!(
            f.metadata.component_directive.as_deref(),
            Some("use server")
        );
    }

    #[test]
    fn no_directive_when_absent() {
        let src = "export function Foo() { return <div/>; }";
        let items = parse_and_extract(src);
        let f = find_by_name(&items, "Foo");
        assert!(f.metadata.component_directive.is_none());
    }

    // ── Component detection ────────────────────────────────────────

    #[test]
    fn button_is_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let btn = find_by_name(&items, "Button");
        assert_eq!(btn.kind, SymbolKind::Component);
        assert!(btn.metadata.is_component);
    }

    #[test]
    fn button_exported() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let btn = find_by_name(&items, "Button");
        assert_eq!(btn.visibility, Visibility::Export);
        assert!(btn.metadata.is_exported);
    }

    #[test]
    fn button_has_props_type() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let btn = find_by_name(&items, "Button");
        assert_eq!(btn.metadata.props_type.as_deref(), Some("ButtonProps"));
    }

    #[test]
    fn button_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let btn = find_by_name(&items, "Button");
        assert!(
            btn.doc_comment.contains("Primary button component"),
            "doc: {:?}",
            btn.doc_comment
        );
    }

    #[test]
    fn button_jsx_elements() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let btn = find_by_name(&items, "Button");
        assert!(
            btn.metadata.jsx_elements.contains(&"button".to_string()),
            "jsx: {:?}",
            btn.metadata.jsx_elements
        );
    }

    // ── Private (non-exported) component ───────────────────────────

    #[test]
    fn sidebar_is_private_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let sb = find_by_name(&items, "Sidebar");
        assert_eq!(sb.kind, SymbolKind::Component);
        assert!(sb.metadata.is_component);
        assert_eq!(sb.visibility, Visibility::Private);
    }

    // ── Arrow component (React.FC) ─────────────────────────────────

    #[test]
    fn usercard_is_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let uc = find_by_name(&items, "UserCard");
        assert_eq!(uc.kind, SymbolKind::Component);
        assert!(uc.metadata.is_component);
    }

    #[test]
    fn usercard_props_type() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let uc = find_by_name(&items, "UserCard");
        assert_eq!(uc.metadata.props_type.as_deref(), Some("UserCardProps"));
    }

    #[test]
    fn usercard_has_hooks() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let uc = find_by_name(&items, "UserCard");
        assert!(
            uc.metadata.hooks_used.contains(&"useCallback".to_string()),
            "hooks: {:?}",
            uc.metadata.hooks_used
        );
    }

    #[test]
    fn usercard_jsx_elements() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let uc = find_by_name(&items, "UserCard");
        assert!(
            uc.metadata.jsx_elements.contains(&"div".to_string()),
            "jsx: {:?}",
            uc.metadata.jsx_elements
        );
    }

    // ── Generic component ──────────────────────────────────────────

    #[test]
    fn list_is_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let list = find_by_name(&items, "List");
        assert_eq!(list.kind, SymbolKind::Component);
        assert!(list.metadata.is_component);
    }

    #[test]
    fn list_has_type_params() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let list = find_by_name(&items, "List");
        assert!(
            list.metadata
                .type_parameters
                .as_deref()
                .is_some_and(|t| t.contains('T')),
            "type_params: {:?}",
            list.metadata.type_parameters
        );
    }

    #[test]
    fn list_props_type() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let list = find_by_name(&items, "List");
        assert!(
            list.metadata
                .props_type
                .as_deref()
                .is_some_and(|p| p.contains("ListProps")),
            "props: {:?}",
            list.metadata.props_type
        );
    }

    // ── Counter (multiple hooks) ───────────────────────────────────

    #[test]
    fn counter_is_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Counter");
        assert_eq!(c.kind, SymbolKind::Component);
        assert!(c.metadata.is_component);
    }

    #[test]
    fn counter_hooks_used() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Counter");
        for hook in &["useState", "useEffect", "useRef", "useMemo"] {
            assert!(
                c.metadata.hooks_used.contains(&(*hook).to_string()),
                "missing hook {hook}: {:?}",
                c.metadata.hooks_used
            );
        }
    }

    #[test]
    fn counter_jsx_elements() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "Counter");
        assert!(
            c.metadata.jsx_elements.contains(&"div".to_string()),
            "jsx: {:?}",
            c.metadata.jsx_elements
        );
        assert!(
            c.metadata.jsx_elements.contains(&"Button".to_string()),
            "jsx: {:?}",
            c.metadata.jsx_elements
        );
    }

    // ── TodoApp (useReducer) ───────────────────────────────────────

    #[test]
    fn todo_app_is_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "TodoApp");
        assert_eq!(t.kind, SymbolKind::Component);
        assert!(t.metadata.is_component);
    }

    #[test]
    fn todo_app_uses_reducer() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "TodoApp");
        assert!(
            t.metadata.hooks_used.contains(&"useReducer".to_string()),
            "hooks: {:?}",
            t.metadata.hooks_used
        );
    }

    // ── Default export component ───────────────────────────────────

    #[test]
    fn app_is_default_export_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let app = find_by_name(&items, "App");
        assert_eq!(app.kind, SymbolKind::Component);
        assert!(app.metadata.is_default_export);
        assert!(app.metadata.is_component);
    }

    #[test]
    fn app_hooks_used() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let app = find_by_name(&items, "App");
        assert!(
            app.metadata.hooks_used.contains(&"useState".to_string()),
            "hooks: {:?}",
            app.metadata.hooks_used
        );
        assert!(
            app.metadata.hooks_used.contains(&"useCallback".to_string()),
            "hooks: {:?}",
            app.metadata.hooks_used
        );
    }

    #[test]
    fn app_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let app = find_by_name(&items, "App");
        assert!(
            app.doc_comment.contains("Main application shell"),
            "doc: {:?}",
            app.doc_comment
        );
    }

    #[test]
    fn app_renders_sidebar() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let app = find_by_name(&items, "App");
        assert!(
            app.metadata.jsx_elements.contains(&"Sidebar".to_string()),
            "jsx: {:?}",
            app.metadata.jsx_elements
        );
    }

    // ── forwardRef component ───────────────────────────────────────

    #[test]
    fn fancyinput_is_forward_ref() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let fi = find_by_name(&items, "FancyInput");
        assert!(fi.metadata.is_forward_ref);
        assert!(fi.metadata.is_component);
        assert_eq!(fi.kind, SymbolKind::Component);
    }

    #[test]
    fn fancyinput_has_jsx() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let fi = find_by_name(&items, "FancyInput");
        assert!(
            fi.metadata.jsx_elements.contains(&"input".to_string()),
            "jsx: {:?}",
            fi.metadata.jsx_elements
        );
    }

    // ── React.memo ─────────────────────────────────────────────────

    #[test]
    fn memo_card_is_memo_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let mc = find_by_name(&items, "MemoCard");
        assert!(mc.metadata.is_memo);
        assert!(mc.metadata.is_component);
        assert_eq!(mc.kind, SymbolKind::Component);
    }

    #[test]
    fn memo_card_has_jsx() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let mc = find_by_name(&items, "MemoCard");
        assert!(
            mc.metadata.jsx_elements.contains(&"div".to_string()),
            "jsx: {:?}",
            mc.metadata.jsx_elements
        );
    }

    #[test]
    fn memo_avatar_is_memo_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let ma = find_by_name(&items, "MemoAvatar");
        assert!(ma.metadata.is_memo);
        assert!(ma.metadata.is_component);
        assert_eq!(ma.kind, SymbolKind::Component);
    }

    #[test]
    fn memo_avatar_has_jsx() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let ma = find_by_name(&items, "MemoAvatar");
        assert!(
            ma.metadata.jsx_elements.contains(&"img".to_string()),
            "jsx: {:?}",
            ma.metadata.jsx_elements
        );
    }

    #[test]
    fn memo_not_forward_ref() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let mc = find_by_name(&items, "MemoCard");
        assert!(!mc.metadata.is_forward_ref);
    }

    // ── React.lazy ─────────────────────────────────────────────────

    #[test]
    fn lazy_settings_is_lazy() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let ls = find_by_name(&items, "LazySettings");
        assert!(ls.metadata.is_lazy);
        assert!(
            !ls.metadata.is_component,
            "lazy import itself is not a component"
        );
    }

    // ── Suspense boundary ──────────────────────────────────────────

    #[test]
    fn page_with_suspense_is_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "PageWithSuspense");
        assert_eq!(p.kind, SymbolKind::Component);
        assert!(p.metadata.is_component);
    }

    #[test]
    fn page_with_suspense_renders_suspense() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "PageWithSuspense");
        assert!(
            p.metadata.jsx_elements.contains(&"Suspense".to_string()),
            "jsx: {:?}",
            p.metadata.jsx_elements
        );
    }

    #[test]
    fn page_with_suspense_renders_lazy_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "PageWithSuspense");
        assert!(
            p.metadata
                .jsx_elements
                .contains(&"LazySettings".to_string()),
            "jsx: {:?}",
            p.metadata.jsx_elements
        );
    }

    #[test]
    fn page_with_suspense_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let p = find_by_name(&items, "PageWithSuspense");
        assert!(
            p.doc_comment.contains("suspense boundary"),
            "doc: {:?}",
            p.doc_comment
        );
    }

    // ── HOC detection ──────────────────────────────────────────────

    #[test]
    fn with_loading_is_hoc() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let hoc = find_by_name(&items, "withLoading");
        assert!(hoc.metadata.is_hoc);
        assert!(!hoc.metadata.is_component, "HOC should not be a component");
    }

    #[test]
    fn with_loading_not_hook() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let hoc = find_by_name(&items, "withLoading");
        assert!(!hoc.metadata.is_hook);
    }

    // ── Class components ───────────────────────────────────────────

    #[test]
    fn error_boundary_is_class_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let eb = find_by_name(&items, "ErrorBoundary");
        assert_eq!(eb.kind, SymbolKind::Component);
        assert!(eb.metadata.is_class_component);
        assert!(eb.metadata.is_component);
    }

    #[test]
    fn error_boundary_is_error_boundary() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let eb = find_by_name(&items, "ErrorBoundary");
        assert!(eb.metadata.is_error_boundary);
    }

    #[test]
    fn error_boundary_has_jsx() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let eb = find_by_name(&items, "ErrorBoundary");
        assert!(
            eb.metadata.jsx_elements.contains(&"div".to_string()),
            "jsx: {:?}",
            eb.metadata.jsx_elements
        );
    }

    #[test]
    fn error_boundary_props_type() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let eb = find_by_name(&items, "ErrorBoundary");
        assert_eq!(eb.metadata.props_type.as_deref(), Some("EBProps"));
    }

    #[test]
    fn error_boundary_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let eb = find_by_name(&items, "ErrorBoundary");
        assert!(
            eb.doc_comment.contains("Error boundary"),
            "doc: {:?}",
            eb.doc_comment
        );
    }

    #[test]
    fn pure_counter_is_class_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let pc = find_by_name(&items, "PureCounter");
        assert_eq!(pc.kind, SymbolKind::Component);
        assert!(pc.metadata.is_class_component);
        assert!(!pc.metadata.is_error_boundary);
    }

    #[test]
    fn pure_counter_is_private() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let pc = find_by_name(&items, "PureCounter");
        assert_eq!(pc.visibility, Visibility::Private);
    }

    #[test]
    fn pure_counter_props_type() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let pc = find_by_name(&items, "PureCounter");
        assert_eq!(pc.metadata.props_type.as_deref(), Some("CounterClassProps"));
    }

    // ── Hook detection ─────────────────────────────────────────────

    #[test]
    fn use_theme_is_hook() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let hook = find_by_name(&items, "useTheme");
        assert!(hook.metadata.is_hook);
        assert!(!hook.metadata.is_component);
    }

    #[test]
    fn use_theme_has_jsdoc() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let hook = find_by_name(&items, "useTheme");
        assert!(
            hook.doc_comment.contains("Custom hook for theme access"),
            "doc: {:?}",
            hook.doc_comment
        );
    }

    #[test]
    fn use_theme_hooks_used() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let hook = find_by_name(&items, "useTheme");
        assert!(
            hook.metadata.hooks_used.contains(&"useContext".to_string()),
            "hooks: {:?}",
            hook.metadata.hooks_used
        );
    }

    #[test]
    fn use_fetch_is_hook() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let hook = find_by_name(&items, "useFetch");
        assert!(hook.metadata.is_hook);
        assert!(!hook.metadata.is_component);
    }

    #[test]
    fn use_fetch_hooks_used() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let hook = find_by_name(&items, "useFetch");
        assert!(
            hook.metadata.hooks_used.contains(&"useState".to_string()),
            "hooks: {:?}",
            hook.metadata.hooks_used
        );
        assert!(
            hook.metadata.hooks_used.contains(&"useEffect".to_string()),
            "hooks: {:?}",
            hook.metadata.hooks_used
        );
    }

    #[test]
    fn use_fetch_has_type_params() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let hook = find_by_name(&items, "useFetch");
        assert!(
            hook.metadata
                .type_parameters
                .as_deref()
                .is_some_and(|t| t.contains('T')),
            "type_params: {:?}",
            hook.metadata.type_parameters
        );
    }

    // ── Non-component items stay unchanged ─────────────────────────

    #[test]
    fn format_date_not_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let f = find_by_name(&items, "formatDate");
        assert_eq!(f.kind, SymbolKind::Function);
        assert!(!f.metadata.is_component);
        assert!(!f.metadata.is_hook);
    }

    #[test]
    fn api_url_not_component() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "API_URL");
        assert_eq!(c.kind, SymbolKind::Const);
        assert!(!c.metadata.is_component);
    }

    #[test]
    fn theme_type_alias_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "Theme");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
    }

    #[test]
    fn status_enum_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let e = find_by_name(&items, "Status");
        assert_eq!(e.kind, SymbolKind::Enum);
        assert!(e.metadata.variants.contains(&"Active".to_string()));
        assert!(e.metadata.variants.contains(&"Inactive".to_string()));
    }

    // ── Props via type alias ───────────────────────────────────────

    #[test]
    fn card_props_type_alias_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let t = find_by_name(&items, "CardProps");
        assert_eq!(t.kind, SymbolKind::TypeAlias);
    }

    // ── Interfaces extracted ───────────────────────────────────────

    #[test]
    fn button_props_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "ButtonProps");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn user_card_props_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "UserCardProps");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn list_props_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "ListProps");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn input_props_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "InputProps");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn avatar_props_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "AvatarProps");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn eb_props_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "EBProps");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn eb_state_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "EBState");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn theme_context_value_interface() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "ThemeContextValue");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn counter_class_props_interface() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "CounterClassProps");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    #[test]
    fn todo_state_interface_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let i = find_by_name(&items, "TodoState");
        assert_eq!(i.kind, SymbolKind::Interface);
    }

    // ── Context const ──────────────────────────────────────────────

    #[test]
    fn theme_context_const_extracted() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        let c = find_by_name(&items, "ThemeContext");
        assert_eq!(c.kind, SymbolKind::Const);
        assert!(!c.metadata.is_component);
    }

    // ── Line numbers ───────────────────────────────────────────────

    #[test]
    fn line_numbers_are_one_based() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        for item in &items {
            assert!(
                item.start_line >= 1,
                "'{}' start_line should be >= 1, got {}",
                item.name,
                item.start_line
            );
            assert!(
                item.end_line >= item.start_line,
                "'{}' end_line {} < start_line {}",
                item.name,
                item.end_line,
                item.start_line
            );
        }
    }

    // ── Naming convention helpers ───────────────────────────────────

    #[test]
    fn hook_name_detection() {
        assert!(is_hook_name("useState"));
        assert!(is_hook_name("useEffect"));
        assert!(is_hook_name("useCustom"));
        assert!(is_hook_name("useReducer"));
        assert!(!is_hook_name("use"));
        assert!(!is_hook_name("useless"));
        assert!(!is_hook_name("User"));
    }

    #[test]
    fn hoc_name_detection() {
        assert!(is_hoc_name("withLoading"));
        assert!(is_hoc_name("withAuth"));
        assert!(!is_hoc_name("with"));
        assert!(!is_hoc_name("without"));
        assert!(!is_hoc_name("Widget"));
    }

    #[test]
    fn component_name_detection() {
        assert!(is_component_name("Button"));
        assert!(is_component_name("App"));
        assert!(!is_component_name("formatDate"));
        assert!(!is_component_name("useState"));
    }

    // ── Return type detection ──────────────────────────────────────

    #[test]
    fn component_return_type_detection() {
        assert!(is_component_return_type(Some("JSX.Element")));
        assert!(is_component_return_type(Some("ReactNode")));
        assert!(is_component_return_type(Some("ReactElement")));
        assert!(is_component_return_type(Some("React.FC<Props>")));
        assert!(!is_component_return_type(Some("string")));
        assert!(!is_component_return_type(None));
    }

    // ── Props extraction from type annotations ─────────────────────

    #[test]
    fn extract_props_from_fc_annotation() {
        assert_eq!(
            extract_props_from_type_annotation(Some("React.FC<UserCardProps>")),
            Some("UserCardProps".to_string())
        );
    }

    #[test]
    fn extract_props_from_function_component_annotation() {
        assert_eq!(
            extract_props_from_type_annotation(Some("React.FunctionComponent<ButtonProps>")),
            Some("ButtonProps".to_string())
        );
    }

    #[test]
    fn extract_props_no_angle_brackets() {
        assert_eq!(extract_props_from_type_annotation(Some("string")), None);
    }

    // ── All items extracted (smoke test) ───────────────────────────

    #[test]
    fn total_item_count() {
        let source = include_str!("../../tests/fixtures/sample.tsx");
        let items = parse_and_extract(source);
        // Expanded fixture has many more items now.
        assert!(
            items.len() >= 28,
            "expected >= 28 items, got {}: {:?}",
            items.len(),
            items.iter().map(|i| &i.name).collect::<Vec<_>>()
        );
    }
}
