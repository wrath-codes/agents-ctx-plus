use serde::Serialize;
use serde_json::Value;

use crate::cli::OutputFormat;
use crate::ui;

pub mod table;

/// Render a serializable response to a string in the requested format.
pub fn render<T: Serialize>(value: &T, format: OutputFormat) -> anyhow::Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(value)?),
        OutputFormat::Table => render_table(value),
        OutputFormat::Raw => Ok(serde_json::to_string(value)?),
    }
}

/// Print a serializable response in the requested format.
pub fn output<T: Serialize>(value: &T, format: OutputFormat) -> anyhow::Result<()> {
    let rendered = render(value, format)?;
    println!("{rendered}");
    Ok(())
}

fn render_table<T: Serialize>(value: &T) -> anyhow::Result<String> {
    let prefs = ui::prefs();
    let options = table::TableOptions {
        max_width: prefs.term_width,
        color: prefs.table_color,
    };

    let value = serde_json::to_value(value)?;
    match value {
        Value::Array(items) => render_array_table(&items),
        Value::Object(map) => {
            let headers = ["key", "value"];
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut rows = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                rows.push(vec![key, value_to_cell(&value)]);
            }
            Ok(table::render_entity_table(&headers, &rows, options))
        }
        scalar => {
            let headers = ["value"];
            let rows = vec![vec![value_to_cell(&scalar)]];
            Ok(table::render_entity_table(&headers, &rows, options))
        }
    }
}

fn render_array_table(items: &[Value]) -> anyhow::Result<String> {
    let prefs = ui::prefs();
    let options = table::TableOptions {
        max_width: prefs.term_width,
        color: prefs.table_color,
    };

    if items.is_empty() {
        return Ok(String::from("(no rows)"));
    }

    let all_objects = items.iter().all(Value::is_object);
    if !all_objects {
        let headers = ["value"];
        let rows = items
            .iter()
            .map(|item| vec![value_to_cell(item)])
            .collect::<Vec<_>>();
        return Ok(table::render_entity_table(&headers, &rows, options));
    }

    let mut headers = Vec::<String>::new();
    for item in items {
        if let Some(map) = item.as_object() {
            for key in map.keys() {
                if !headers.contains(key) {
                    headers.push(key.clone());
                }
            }
        }
    }

    if headers.is_empty() {
        return Ok(String::from("(no columns)"));
    }

    headers.sort();

    let header_refs = headers.iter().map(String::as_str).collect::<Vec<_>>();
    let rows = items
        .iter()
        .filter_map(Value::as_object)
        .map(|map| {
            headers
                .iter()
                .map(|header| {
                    map.get(header)
                        .map_or_else(|| String::from("-"), value_to_cell)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Ok(table::render_entity_table(&header_refs, &rows, options))
}

fn value_to_cell(value: &Value) -> String {
    match value {
        Value::Null => String::from("null"),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| String::from("<invalid-json>")),
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::{render, table::render_entity_table};
    use crate::cli::OutputFormat;

    #[derive(Serialize)]
    struct Example {
        id: &'static str,
        value: u32,
    }

    #[test]
    fn json_render_is_valid_json() {
        let value = Example { id: "x", value: 7 };
        let out = render(&value, OutputFormat::Json).expect("json render should work");
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("json should parse");
        assert_eq!(parsed["id"], "x");
        assert_eq!(parsed["value"], 7);
    }

    #[test]
    fn raw_render_is_single_line_json() {
        let value = Example { id: "x", value: 7 };
        let out = render(&value, OutputFormat::Raw).expect("raw render should work");
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("json should parse");
        assert_eq!(parsed["id"], "x");
        assert!(!out.contains('\n'));
    }

    #[test]
    fn table_render_for_object_is_tabular() {
        let value = Example { id: "x", value: 7 };
        let out = render(&value, OutputFormat::Table).expect("table render should work");
        assert!(out.lines().next().is_some_and(|line| line.contains("key")));
        assert!(out.contains("id"));
        assert!(out.contains("value"));
    }

    #[test]
    fn table_alignment_handles_mixed_widths() {
        let headers = ["id", "status", "title"];
        let rows = vec![
            vec!["tsk-1".to_string(), "todo".to_string(), "short".to_string()],
            vec![
                "tsk-200".to_string(),
                "in_progress".to_string(),
                "a much longer title".to_string(),
            ],
        ];

        let table = render_entity_table(
            &headers,
            &rows,
            super::table::TableOptions {
                max_width: None,
                color: false,
            },
        );
        let lines: Vec<&str> = table.lines().collect();

        assert!(lines.len() >= 4);
        assert!(lines[0].contains("id"));
        assert!(lines[0].contains("status"));
        assert!(lines[0].contains("title"));
        assert!(lines[1].chars().all(|c| c == '-'));
    }
}
