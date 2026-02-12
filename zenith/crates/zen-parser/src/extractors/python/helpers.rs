use ast_grep_core::Node;

use crate::types::Visibility;

pub(super) const EXCEPTION_BASE_CLASSES: &[&str] = &[
    "Exception",
    "BaseException",
    "ValueError",
    "TypeError",
    "RuntimeError",
    "IOError",
    "OSError",
    "KeyError",
    "IndexError",
    "AttributeError",
    "NotImplementedError",
    "StopIteration",
    "ArithmeticError",
    "LookupError",
    "EnvironmentError",
];

pub(super) fn extract_decorators<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    node.children()
        .filter(|c| c.kind().as_ref() == "decorator")
        .map(|c| {
            let text = c.text().to_string();
            text.trim_start_matches('@').trim().to_string()
        })
        .collect()
}

pub(super) fn is_exception_subclass(base_classes: &[String]) -> bool {
    base_classes
        .iter()
        .any(|b| EXCEPTION_BASE_CLASSES.contains(&b.as_str()) || b.ends_with("Error"))
}

/// Check if any decorator matches the given suffixes (handles dotted paths).
pub(super) fn decorator_matches_any(decorators: &[String], suffixes: &[&str]) -> bool {
    decorators.iter().any(|d| {
        let base = d.split('(').next().unwrap_or(d);
        suffixes
            .iter()
            .any(|s| base == *s || base.ends_with(&format!(".{s}")))
    })
}

pub(super) fn decorator_matches(decorators: &[String], suffix: &str) -> bool {
    decorator_matches_any(decorators, &[suffix])
}

/// Python visibility rules:
/// - `__dunder__` (starts AND ends with `__`) -> Public
/// - `__name_mangled` (starts with `__`, no trailing `__`) -> Private
/// - `_protected` (starts with single `_`) -> Protected
/// - everything else -> Public
pub(super) fn python_visibility(name: &str) -> Visibility {
    if name.starts_with("__") && name.ends_with("__") && name.len() > 4 {
        Visibility::Public
    } else if name.starts_with("__") {
        Visibility::Private
    } else if name.starts_with('_') {
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

pub(super) fn extract_python_parameters<D: ast_grep_core::Doc>(node: &Node<D>) -> Vec<String> {
    let Some(params) = node.field("parameters") else {
        return Vec::new();
    };
    params
        .children()
        .filter(|c| {
            let k = c.kind();
            k.as_ref() != "(" && k.as_ref() != ")" && k.as_ref() != ","
        })
        .map(|c| c.text().to_string())
        .collect()
}

/// Detect if a function is a generator by looking for `yield` statements,
/// but only in the function's own body - not in nested function definitions.
pub(super) fn detect_generator<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    let Some(body) = node.field("body") else {
        return false;
    };
    has_yield_shallow(&body)
}

/// Shallow recursive search for yield, stopping at nested function boundaries.
fn has_yield_shallow<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    for child in node.children() {
        let k = child.kind();
        let kind = k.as_ref();
        // Stop at nested function definitions
        if kind == "function_definition" || kind == "decorated_definition" {
            continue;
        }
        if kind == "yield" || kind == "yield_expression" {
            return true;
        }
        if has_yield_shallow(&child) {
            return true;
        }
    }
    false
}
