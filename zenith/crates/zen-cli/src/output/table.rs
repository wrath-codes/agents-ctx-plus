/// Render a simple aligned table for string rows.
#[must_use]
pub fn render_entity_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .map(std::string::String::len)
                .max()
                .unwrap_or(0)
                .max(header.len())
        })
        .collect();

    let header_line = headers
        .iter()
        .zip(widths.iter())
        .map(|(header, width)| format!("{header:<width$}"))
        .collect::<Vec<_>>()
        .join("  ");

    let divider = "-".repeat(header_line.len());

    let row_lines = rows
        .iter()
        .map(|row| {
            row.iter()
                .zip(widths.iter())
                .map(|(value, width)| format!("{value:<width$}"))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect::<Vec<_>>();

    let mut lines = Vec::with_capacity(2 + row_lines.len());
    lines.push(header_line);
    lines.push(divider);
    lines.extend(row_lines);
    lines.join("\n")
}

/// Print a simple aligned table.
pub fn print_entity_table(headers: &[&str], rows: &[Vec<String>]) {
    println!("{}", render_entity_table(headers, rows));
}
