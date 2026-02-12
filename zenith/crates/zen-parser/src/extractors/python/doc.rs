use ast_grep_core::Node;

use crate::types::DocSections;

/// Extract the module-level docstring (first string expression in module body).
pub(super) fn extract_module_docstring<D: ast_grep_core::Doc>(root: &Node<D>) -> Option<String> {
    let first = root.children().next()?;
    if first.kind().as_ref() != "expression_statement" {
        return None;
    }
    let string_node = first.children().find(|c| c.kind().as_ref() == "string")?;
    let text = string_node.text().to_string();
    let doc = text
        .trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim_start_matches("'''")
        .trim_end_matches("'''")
        .trim();
    if doc.is_empty() {
        return None;
    }
    Some(doc.to_string())
}

pub(super) fn extract_docstring<D: ast_grep_core::Doc>(node: &Node<D>) -> String {
    let Some(body) = node.field("body") else {
        return String::new();
    };

    let Some(first_child) = body.children().next() else {
        return String::new();
    };

    if first_child.kind().as_ref() != "expression_statement" {
        return String::new();
    }

    let Some(s) = first_child
        .children()
        .find(|c| c.kind().as_ref() == "string")
    else {
        return String::new();
    };

    let text = s.text().to_string();
    text.trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim_start_matches("'''")
        .trim_end_matches("'''")
        .trim()
        .to_string()
}

pub(super) fn parse_python_doc_sections(doc: &str) -> DocSections {
    if doc.is_empty() {
        return DocSections::default();
    }

    // Try Google-style first, then Sphinx-style, then NumPy-style
    let mut sections = parse_google_style(doc);
    if sections_empty(&sections) {
        sections = parse_sphinx_style(doc);
    }
    if sections_empty(&sections) {
        sections = parse_numpy_style(doc);
    }
    sections
}

fn sections_empty(s: &DocSections) -> bool {
    s.args.is_empty() && s.returns.is_none() && s.raises.is_empty() && s.yields.is_none()
}

fn parse_google_style(doc: &str) -> DocSections {
    let mut sections = DocSections::default();
    let mut current_section: Option<&str> = None;
    let mut current_content = String::new();

    for line in doc.lines() {
        let trimmed = line.trim();

        if trimmed == "Args:" || trimmed == "Arguments:" || trimmed == "Parameters:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("args");
            current_content.clear();
        } else if trimmed == "Returns:" || trimmed == "Return:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("returns");
            current_content.clear();
        } else if trimmed == "Raises:" || trimmed == "Exceptions:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("raises");
            current_content.clear();
        } else if trimmed == "Yields:" || trimmed == "Yield:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("yields");
            current_content.clear();
        } else if trimmed == "Examples:" || trimmed == "Example:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("examples");
            current_content.clear();
        } else if trimmed == "Notes:" || trimmed == "Note:" {
            flush_google_section(&mut sections, current_section, &current_content);
            current_section = Some("notes");
            current_content.clear();
        } else if current_section.is_some() {
            if !current_content.is_empty() {
                current_content.push('\n');
            }
            current_content.push_str(trimmed);
        }
    }
    flush_google_section(&mut sections, current_section, &current_content);
    sections
}

fn flush_google_section(sections: &mut DocSections, heading: Option<&str>, content: &str) {
    let content = content.trim();
    if content.is_empty() {
        return;
    }
    match heading {
        Some("args") => {
            for param in parse_param_block(content) {
                sections.args.insert(param.0, param.1);
            }
        }
        Some("returns") => sections.returns = Some(content.to_string()),
        Some("raises") => {
            for exc in parse_param_block(content) {
                sections.raises.insert(exc.0, exc.1);
            }
        }
        Some("yields") => sections.yields = Some(content.to_string()),
        Some("examples") => sections.examples = Some(content.to_string()),
        Some("notes") => sections.notes = Some(content.to_string()),
        _ => {}
    }
}

fn parse_param_block(block: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let mut current_name = String::new();
    let mut current_desc = String::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if let Some((name, desc)) = trimmed.split_once(':') {
            if !current_name.is_empty() {
                params.push((current_name.clone(), current_desc.trim().to_string()));
            }
            current_name = name.trim().to_string();
            current_desc = desc.trim().to_string();
        } else if !current_name.is_empty() {
            current_desc.push(' ');
            current_desc.push_str(trimmed);
        }
    }
    if !current_name.is_empty() {
        params.push((current_name, current_desc.trim().to_string()));
    }
    params
}

fn parse_sphinx_style(doc: &str) -> DocSections {
    let mut sections = DocSections::default();

    for line in doc.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(":param ") {
            if let Some((name, desc)) = rest.split_once(':') {
                sections
                    .args
                    .insert(name.trim().to_string(), desc.trim().to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix(":returns:") {
            sections.returns = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix(":return:") {
            sections.returns = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix(":raises ")
            && let Some((exc, desc)) = rest.split_once(':')
        {
            sections
                .raises
                .insert(exc.trim().to_string(), desc.trim().to_string());
        }
    }
    sections
}

/// Parse NumPy-style docstrings.
///
/// `NumPy` uses section headers underlined with dashes:
/// ```text
/// Parameters
/// ----------
/// x : float
///     Description.
/// ```
pub(super) fn parse_numpy_style(doc: &str) -> DocSections {
    let mut sections = DocSections::default();
    let lines: Vec<&str> = doc.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Check if next line is a dash underline (NumPy section marker)
        let is_section_header = i + 1 < lines.len()
            && !trimmed.is_empty()
            && lines[i + 1].trim().chars().all(|c| c == '-')
            && !lines[i + 1].trim().is_empty();

        if is_section_header {
            let section_name = trimmed;
            i += 2; // Skip header + underline

            // Collect section content until next section or end
            let mut content = String::new();
            while i < lines.len() {
                let line = lines[i].trim();
                // Check if this is another section header
                if i + 1 < lines.len()
                    && !line.is_empty()
                    && lines[i + 1].trim().chars().all(|c| c == '-')
                    && !lines[i + 1].trim().is_empty()
                {
                    break;
                }
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(line);
                i += 1;
            }

            let content = content.trim().to_string();
            match section_name {
                "Parameters" | "Params" | "Args" => {
                    for (name, desc) in parse_numpy_param_block(&content) {
                        sections.args.insert(name, desc);
                    }
                }
                "Returns" | "Return" => sections.returns = Some(content),
                "Raises" | "Exceptions" => {
                    for (name, desc) in parse_numpy_param_block(&content) {
                        sections.raises.insert(name, desc);
                    }
                }
                "Yields" | "Yield" => sections.yields = Some(content),
                "Examples" | "Example" => sections.examples = Some(content),
                "Notes" | "Note" => sections.notes = Some(content),
                _ => {}
            }
        } else {
            i += 1;
        }
    }
    sections
}

/// Parse a NumPy-style parameter block (entries like `name : type\n    desc`).
fn parse_numpy_param_block(block: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let mut current_name = String::new();
    let mut current_desc = String::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // NumPy param lines: "name : type" or just "name"
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if !current_name.is_empty() {
                params.push((current_name.clone(), current_desc.trim().to_string()));
            }
            current_name = trimmed
                .split(':')
                .next()
                .unwrap_or(trimmed)
                .trim()
                .to_string();
            current_desc = String::new();
        } else {
            if !current_desc.is_empty() {
                current_desc.push(' ');
            }
            current_desc.push_str(trimmed);
        }
    }
    if !current_name.is_empty() {
        params.push((current_name, current_desc.trim().to_string()));
    }
    params
}
