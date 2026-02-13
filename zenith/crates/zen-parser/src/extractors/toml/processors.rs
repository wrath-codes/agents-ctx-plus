use ast_grep_core::Node;
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::extractors::helpers::extract_source;
use crate::types::{CommonMetadataExt, ParsedItem, SymbolKind, SymbolMetadata, Visibility};

use super::toml_helpers;

struct TomlContext {
    array_table_counts: HashMap<String, usize>,
    seen_table_kinds: HashMap<String, TableKind>,
    seen_keys: HashSet<String>,
    comment_lines: HashMap<u32, String>,
    has_comments: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TableKind {
    Table,
    Array,
}

pub(super) fn extract_document<D: ast_grep_core::Doc>(root: &Node<D>) -> Vec<ParsedItem> {
    let mut items = Vec::new();
    let source = root.text().to_string();
    let mut ctx = TomlContext {
        array_table_counts: HashMap::new(),
        seen_table_kinds: HashMap::new(),
        seen_keys: HashSet::new(),
        comment_lines: collect_comments_by_line(&source),
        has_comments: contains_comment(root),
    };

    let mut metadata = SymbolMetadata::default();
    metadata.push_attribute("toml:kind:document");
    if ctx.has_comments {
        metadata.push_attribute("toml:nonstandard:comments");
    }
    let mut root_item = build_item(
        root,
        SymbolKind::Module,
        "$".to_string(),
        "$".to_string(),
        metadata,
    );
    attach_comments(&mut root_item, &ctx);
    items.push(root_item);

    for child in root.children() {
        match child.kind().as_ref() {
            "pair" => collect_pair(&child, "", "$", &mut ctx, &mut items),
            "table" => collect_table(&child, false, &mut ctx, &mut items),
            "table_array_element" => collect_table(&child, true, &mut ctx, &mut items),
            _ => {}
        }
    }

    items
}

fn collect_table<D: ast_grep_core::Doc>(
    node: &Node<D>,
    is_array_table: bool,
    ctx: &mut TomlContext,
    out: &mut Vec<ParsedItem>,
) {
    let mut path_parts =
        toml_helpers::key_parts_from_table_text(&node.text(), is_array_table).unwrap_or_default();
    if path_parts.is_empty() {
        for child in node.children() {
            if child.kind().as_ref() == "pair" {
                break;
            }
            let kind = child.kind();
            let kind_ref = kind.as_ref();
            if toml_helpers::is_key_kind(kind_ref) {
                path_parts.extend(toml_helpers::key_parts(&child));
            }
        }
    }

    if path_parts.is_empty() {
        return;
    }

    let path = path_parts.join(".");
    let (table_name, owner_name) = if is_array_table {
        let idx = ctx.array_table_counts.entry(path.clone()).or_insert(0);
        let name = format!("{path}[{}]", *idx);
        *idx += 1;
        (name, path.clone())
    } else {
        (path.clone(), "$".to_string())
    };

    let mut duplicate_table = false;
    let mut conflicting_kind = false;
    let expected_kind = if is_array_table {
        TableKind::Array
    } else {
        TableKind::Table
    };
    if let Some(prev) = ctx.seen_table_kinds.get(&path).copied() {
        if prev == expected_kind {
            duplicate_table = true;
        } else {
            conflicting_kind = true;
        }
    }
    ctx.seen_table_kinds.insert(path.clone(), expected_kind);

    let mut key_conflicts = Vec::new();
    if ctx.seen_keys.contains(&path) {
        key_conflicts.push(path.clone());
    }
    for prefix in toml_helpers::path_prefixes(&path) {
        if ctx.seen_keys.contains(&prefix) {
            key_conflicts.push(prefix);
        }
    }

    let mut metadata = SymbolMetadata::default();
    metadata.set_owner_name(Some(owner_name));
    metadata.set_owner_kind(Some(SymbolKind::Module));
    metadata.push_attribute(if is_array_table {
        "toml:kind:table_array".to_string()
    } else {
        "toml:kind:table".to_string()
    });
    if is_array_table {
        let index = table_name
            .rsplit('[')
            .next()
            .and_then(|tail| tail.strip_suffix(']'))
            .unwrap_or("0");
        metadata.push_attribute(format!("toml:table_array:path:{path}"));
        metadata.push_attribute(format!("toml:table_array:index:{index}"));
    }
    if duplicate_table {
        metadata.push_attribute(format!("toml:duplicate_table:{path}"));
    }
    if conflicting_kind {
        metadata.push_attribute(format!("toml:table_kind_conflict:{path}"));
    }
    for conflict in key_conflicts {
        metadata.push_attribute(format!("toml:table_key_conflict:{conflict}"));
    }
    if ctx.has_comments {
        metadata.push_attribute("toml:nonstandard:comments");
    }

    let mut item = build_item(
        node,
        SymbolKind::Module,
        table_name.clone(),
        if is_array_table {
            format!("[[{path}]]")
        } else {
            format!("[{path}]")
        },
        metadata,
    );
    attach_comments(&mut item, ctx);
    out.push(item);

    for child in node.children() {
        if child.kind().as_ref() == "pair" {
            collect_pair(&child, &table_name, &table_name, ctx, out);
        }
    }
}

fn collect_pair<D: ast_grep_core::Doc>(
    pair: &Node<D>,
    parent_path: &str,
    owner_name: &str,
    ctx: &mut TomlContext,
    out: &mut Vec<ParsedItem>,
) {
    let mut key_node = pair.field("key");
    let mut value_node = pair.field("value");

    if key_node.is_none() || value_node.is_none() {
        for child in pair.children() {
            let kind = child.kind();
            let kind_ref = kind.as_ref();
            if key_node.is_none() && toml_helpers::is_key_kind(kind_ref) {
                key_node = Some(child.clone());
                continue;
            }
            if value_node.is_none() && toml_helpers::is_value_kind(kind_ref) {
                value_node = Some(child);
            }
        }
    }

    let (Some(key_node), Some(value_node)) = (key_node, value_node) else {
        return;
    };

    let key_parts = toml_helpers::key_parts_from_pair_text(&pair.text())
        .unwrap_or_else(|| toml_helpers::key_parts(&key_node));
    if key_parts.is_empty() {
        return;
    }
    let key_name = key_parts.join(".");
    let full_path = toml_helpers::join_path(parent_path, &key_parts);
    let duplicate_key = !ctx.seen_keys.insert(full_path.clone());
    let shadows_table = ctx.seen_table_kinds.contains_key(&full_path);
    let parent_key_conflict = toml_helpers::path_prefixes(&full_path)
        .into_iter()
        .find(|prefix| ctx.seen_keys.contains(prefix));
    let child_key_conflict = ctx
        .seen_keys
        .iter()
        .any(|existing| existing.starts_with(&(full_path.clone() + ".")));

    let mut metadata = SymbolMetadata::default();
    metadata.set_owner_name(Some(owner_name.to_string()));
    metadata.set_owner_kind(Some(SymbolKind::Module));
    metadata.set_return_type(Some(toml_helpers::toml_value_type(&value_node)));
    metadata.push_attribute(format!("toml:key:{key_name}"));
    if let Some(norm) =
        toml_helpers::normalized_scalar(value_node.kind().as_ref(), &value_node.text())
    {
        metadata.push_attribute(format!("toml:value_normalized:{norm}"));
    }
    if duplicate_key {
        metadata.push_attribute(format!("toml:duplicate_key:{full_path}"));
    }
    if shadows_table {
        metadata.push_attribute(format!("toml:key_table_conflict:{full_path}"));
    }
    if let Some(prefix) = parent_key_conflict {
        metadata.push_attribute(format!("toml:key_parent_conflict:{prefix}"));
    }
    if child_key_conflict {
        metadata.push_attribute(format!("toml:key_child_conflict:{full_path}"));
    }
    if ctx.has_comments {
        metadata.push_attribute("toml:nonstandard:comments");
    }
    enrich_dependency_metadata(&full_path, &value_node, &mut metadata);
    enrich_shape_metadata(&value_node, &mut metadata);

    let mut item = build_item(
        pair,
        SymbolKind::Property,
        full_path.clone(),
        key_name,
        metadata,
    );
    attach_comments(&mut item, ctx);
    out.push(item);

    collect_value(&value_node, &full_path, ctx, out);
}

fn collect_value<D: ast_grep_core::Doc>(
    value: &Node<D>,
    path: &str,
    ctx: &mut TomlContext,
    out: &mut Vec<ParsedItem>,
) {
    match value.kind().as_ref() {
        "inline_table" => {
            for child in value.children() {
                if child.kind().as_ref() == "pair" {
                    collect_pair(&child, path, path, ctx, out);
                }
            }
        }
        "array" => {
            let mut idx = 0usize;
            for child in value.children() {
                let kind = child.kind();
                let kind_ref = kind.as_ref();
                if !toml_helpers::is_value_kind(kind_ref) {
                    continue;
                }

                let element_path = format!("{path}[{idx}]");
                idx += 1;

                if matches!(kind_ref, "inline_table" | "array") {
                    let mut metadata = SymbolMetadata::default();
                    metadata.set_owner_name(Some(path.to_string()));
                    metadata.set_owner_kind(Some(SymbolKind::Module));
                    metadata.set_return_type(Some(toml_helpers::toml_value_type(&child)));
                    metadata.push_attribute("toml:array_element");
                    if let Some(norm) = toml_helpers::normalized_scalar(kind_ref, &child.text()) {
                        metadata.push_attribute(format!("toml:value_normalized:{norm}"));
                    }
                    if ctx.has_comments {
                        metadata.push_attribute("toml:nonstandard:comments");
                    }
                    enrich_array_dependency_metadata(path, &child, &mut metadata);
                    let mut item = build_item(
                        &child,
                        SymbolKind::Property,
                        element_path.clone(),
                        element_path.clone(),
                        metadata,
                    );
                    attach_comments(&mut item, ctx);
                    out.push(item);
                    collect_value(&child, &element_path, ctx, out);
                    continue;
                }

                let mut metadata = SymbolMetadata::default();
                metadata.set_owner_name(Some(path.to_string()));
                metadata.set_owner_kind(Some(SymbolKind::Module));
                metadata.set_return_type(Some(toml_helpers::toml_value_type(&child)));
                metadata.push_attribute("toml:array_element");
                if let Some(norm) = toml_helpers::normalized_scalar(kind_ref, &child.text()) {
                    metadata.push_attribute(format!("toml:value_normalized:{norm}"));
                }
                if ctx.has_comments {
                    metadata.push_attribute("toml:nonstandard:comments");
                }
                enrich_array_dependency_metadata(path, &child, &mut metadata);
                let mut item = build_item(
                    &child,
                    SymbolKind::Property,
                    element_path.clone(),
                    element_path,
                    metadata,
                );
                attach_comments(&mut item, ctx);
                out.push(item);
            }
        }
        _ => {}
    }
}

fn enrich_shape_metadata<D: ast_grep_core::Doc>(value: &Node<D>, metadata: &mut SymbolMetadata) {
    match value.kind().as_ref() {
        "inline_table" => {
            let count = value
                .children()
                .filter(|child| child.kind().as_ref() == "pair")
                .count();
            metadata.push_attribute(format!("toml:object_keys:{count}"));
        }
        "array" => {
            let mut count = 0usize;
            let mut element_types = BTreeSet::new();
            for child in value.children() {
                let kind = child.kind();
                let kind_ref = kind.as_ref();
                if toml_helpers::is_value_kind(kind_ref) {
                    count += 1;
                    element_types.insert(toml_helpers::toml_value_type(&child));
                }
            }
            metadata.push_attribute(format!("toml:array_count:{count}"));
            if element_types.is_empty() {
                metadata.push_attribute("toml:array_elements:empty");
            } else {
                if element_types.len() > 1 {
                    metadata.push_attribute("toml:array_mixed");
                }
                metadata.push_attribute(format!(
                    "toml:array_elements:{}",
                    element_types.into_iter().collect::<Vec<_>>().join("|")
                ));
            }
        }
        _ => {}
    }
}

fn enrich_dependency_metadata<D: ast_grep_core::Doc>(
    full_path: &str,
    value: &Node<D>,
    metadata: &mut SymbolMetadata,
) {
    let Some((scope, dep_name)) = toml_helpers::dependency_from_path(full_path) else {
        return;
    };

    metadata.push_attribute("toml:dependency");
    metadata.push_attribute(format!("toml:dep_scope:{scope}"));
    metadata.push_attribute(format!("toml:dep_name:{dep_name}"));

    match value.kind().as_ref() {
        "string" => {
            if let Some(req) = toml_helpers::normalized_scalar("string", &value.text()) {
                metadata.push_attribute(format!("toml:dep_req:{req}"));
            }
        }
        "inline_table" => {
            for child in value.children() {
                if child.kind().as_ref() != "pair" {
                    continue;
                }
                let pair_text = child.text().to_string();
                if pair_text.trim_start().starts_with("version") {
                    if let Some(raw) = pair_text.split('=').nth(1)
                        && let Some(req) = toml_helpers::normalized_scalar("string", raw)
                        && !req.is_empty()
                    {
                        metadata.push_attribute(format!("toml:dep_req:{req}"));
                    }
                    continue;
                }

                if pair_text.trim_start().starts_with("path") {
                    metadata.push_attribute("toml:dep_source:path");
                }
                if pair_text.trim_start().starts_with("git") {
                    metadata.push_attribute("toml:dep_source:git");
                }
                if pair_text.trim_start().starts_with("workspace") {
                    metadata.push_attribute("toml:dep_source:workspace");
                }
                if pair_text.trim_start().starts_with("registry") {
                    metadata.push_attribute("toml:dep_source:registry");
                }
                if pair_text.trim_start().starts_with("optional") && pair_text.contains("= true") {
                    metadata.push_attribute("toml:dep_optional");
                }
                if pair_text.trim_start().starts_with("package") {
                    let package = pair_text
                        .split('=')
                        .nth(1)
                        .map(str::trim)
                        .map(|s| s.trim_matches('"').trim_matches('\''))
                        .unwrap_or_default();
                    if !package.is_empty() {
                        metadata.push_attribute(format!("toml:dep_package:{package}"));
                    }
                }
            }
        }
        _ => {}
    }
}

fn enrich_array_dependency_metadata<D: ast_grep_core::Doc>(
    array_path: &str,
    value: &Node<D>,
    metadata: &mut SymbolMetadata,
) {
    if value.kind().as_ref() != "string" {
        return;
    }

    let dep_scope = if array_path == "project.dependencies" {
        Some("pep621:dependencies".to_string())
    } else {
        array_path
            .strip_prefix("project.optional-dependencies.")
            .and_then(|s| s.split('.').next())
            .map(|group| format!("pep621:optional:{group}"))
    };

    let Some(dep_scope) = dep_scope else {
        return;
    };

    let Some(norm) = toml_helpers::normalized_scalar("string", &value.text()) else {
        return;
    };
    let Some((name, req)) = toml_helpers::pep508_req_from_string(&norm) else {
        return;
    };

    metadata.push_attribute("toml:dependency");
    metadata.push_attribute(format!("toml:dep_scope:{dep_scope}"));
    metadata.push_attribute(format!("toml:dep_name:{name}"));
    if !req.is_empty() {
        metadata.push_attribute(format!("toml:dep_req:{req}"));
    }
}

fn attach_comments(item: &mut ParsedItem, ctx: &TomlContext) {
    if ctx.comment_lines.is_empty() {
        return;
    }

    let mut docs = Vec::new();
    let mut line = item.start_line.saturating_sub(1);

    while line > 0 {
        if let Some(c) = ctx.comment_lines.get(&line) {
            docs.push(c.clone());
            line -= 1;
        } else {
            break;
        }
    }

    docs.reverse();
    if let Some(inline) = ctx.comment_lines.get(&item.start_line)
        && !docs.iter().any(|d| d == inline)
    {
        docs.push(inline.clone());
    }

    if !docs.is_empty() {
        item.doc_comment = docs.join("\n");
    }
}

fn collect_comments_by_line(source: &str) -> HashMap<u32, String> {
    let mut out = HashMap::new();
    let mut in_basic_ml = false;
    let mut in_literal_ml = false;

    for (idx, line) in source.lines().enumerate() {
        let line_no = idx as u32 + 1;
        if let Some(comment) = extract_comment_text(line, &mut in_basic_ml, &mut in_literal_ml) {
            out.insert(line_no, comment);
        }
    }
    out
}

fn extract_comment_text(
    line: &str,
    in_basic_ml: &mut bool,
    in_literal_ml: &mut bool,
) -> Option<String> {
    let mut in_basic = false;
    let mut in_literal = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if *in_basic_ml {
            if line[i..].starts_with("\"\"\"") {
                *in_basic_ml = false;
                i += 3;
            } else {
                i += 1;
            }
            continue;
        }
        if *in_literal_ml {
            if line[i..].starts_with("'''") {
                *in_literal_ml = false;
                i += 3;
            } else {
                i += 1;
            }
            continue;
        }

        let ch = bytes[i] as char;
        if in_basic {
            if escaped {
                escaped = false;
                i += 1;
                continue;
            }
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_basic = false;
            }
            i += 1;
            continue;
        }

        if in_literal {
            if ch == '\'' {
                in_literal = false;
            }
            i += 1;
            continue;
        }

        if line[i..].starts_with("\"\"\"") {
            *in_basic_ml = true;
            i += 3;
            continue;
        }
        if line[i..].starts_with("'''") {
            *in_literal_ml = true;
            i += 3;
            continue;
        }

        match ch {
            '"' => {
                in_basic = true;
                i += 1;
            }
            '\'' => {
                in_literal = true;
                i += 1;
            }
            '#' => {
                let text = line[i + 1..].trim();
                return if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                };
            }
            _ => i += 1,
        }
    }

    None
}

fn contains_comment<D: ast_grep_core::Doc>(node: &Node<D>) -> bool {
    if node.kind().as_ref() == "comment" {
        return true;
    }
    node.children().any(|child| contains_comment(&child))
}

fn build_item<D: ast_grep_core::Doc>(
    node: &Node<D>,
    kind: SymbolKind,
    name: String,
    signature: String,
    metadata: SymbolMetadata,
) -> ParsedItem {
    let text = node.text().to_string();
    let start_line = node.start_pos().line() as u32 + 1;
    let trimmed = text.trim_end_matches(['\r', '\n']);
    let line_count = if trimmed.is_empty() {
        1
    } else {
        trimmed.lines().count() as u32
    };

    ParsedItem {
        kind,
        name,
        signature,
        source: extract_source(node, 40),
        doc_comment: String::new(),
        start_line,
        end_line: start_line + line_count - 1,
        visibility: Visibility::Public,
        metadata,
    }
}
