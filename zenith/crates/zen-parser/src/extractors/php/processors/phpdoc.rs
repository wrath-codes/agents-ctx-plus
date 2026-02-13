#[derive(Default)]
pub struct PhpDocData {
    pub return_type: Option<String>,
    pub param_types: std::collections::HashMap<String, String>,
    pub templates: Vec<String>,
    pub extends: Vec<String>,
    pub implements: Vec<String>,
    pub var_type: Option<String>,
    pub tags: Vec<String>,
}

pub fn parse_phpdoc(doc: &str) -> PhpDocData {
    let mut out = PhpDocData::default();

    for line in doc.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('@') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@param ") {
            let mut parts = rest.split_whitespace();
            let ty = parts.next().unwrap_or_default().trim();
            let name = parts
                .next()
                .unwrap_or_default()
                .trim_start_matches('$')
                .trim();
            if !name.is_empty() && !ty.is_empty() {
                out.param_types.insert(name.to_string(), ty.to_string());
                out.tags.push(format!("phpdoc:param:{name}:{ty}"));
            } else {
                out.tags.push(format!("phpdoc:param:{}", rest.trim()));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@return ") {
            let ty = rest.split_whitespace().next().unwrap_or_default().trim();
            if ty.is_empty() {
                out.tags.push(format!("phpdoc:return:{}", rest.trim()));
            } else {
                out.return_type = Some(ty.to_string());
                out.tags.push(format!("phpdoc:return:{ty}"));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@var ") {
            let ty = rest.split_whitespace().next().unwrap_or_default().trim();
            if ty.is_empty() {
                out.tags.push(format!("phpdoc:var:{}", rest.trim()));
            } else {
                out.var_type = Some(ty.to_string());
                out.tags.push(format!("phpdoc:var:{ty}"));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@throws ") {
            let ty = rest.split_whitespace().next().unwrap_or_default().trim();
            if ty.is_empty() {
                out.tags.push(format!("phpdoc:throws:{}", rest.trim()));
            } else {
                out.tags.push(format!("phpdoc:throws:{ty}"));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@template ") {
            let tpl = rest.split_whitespace().next().unwrap_or_default().trim();
            if tpl.is_empty() {
                out.tags.push(format!("phpdoc:template:{}", rest.trim()));
            } else {
                out.templates.push(tpl.to_string());
                out.tags.push(format!("phpdoc:template:{tpl}"));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@extends ") {
            let ty = rest.split_whitespace().next().unwrap_or_default().trim();
            if ty.is_empty() {
                out.tags.push(format!("phpdoc:extends:{}", rest.trim()));
            } else {
                out.extends.push(ty.to_string());
                out.tags.push(format!("phpdoc:extends:{ty}"));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@implements ") {
            let ty = rest.split_whitespace().next().unwrap_or_default().trim();
            if ty.is_empty() {
                out.tags.push(format!("phpdoc:implements:{}", rest.trim()));
            } else {
                out.implements.push(ty.to_string());
                out.tags.push(format!("phpdoc:implements:{ty}"));
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@psalm-") {
            out.tags.push(format!("phpdoc:psalm:{}", rest.trim()));
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@phpstan-") {
            out.tags.push(format!("phpdoc:phpstan:{}", rest.trim()));
            continue;
        }

        out.tags.push(format!("phpdoc:{trimmed}"));
    }

    out
}
