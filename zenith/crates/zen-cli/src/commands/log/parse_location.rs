/// Parsed implementation location.
#[derive(Debug)]
pub struct ParsedLocation {
    pub file_path: String,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
}

pub fn parse_location(raw: &str) -> anyhow::Result<ParsedLocation> {
    let (file_path, range) = match raw.split_once('#') {
        Some((path, lines)) => (path.trim(), Some(lines.trim())),
        None => (raw.trim(), None),
    };

    if file_path.is_empty() {
        anyhow::bail!("location must include a file path");
    }

    let (start_line, end_line) = match range {
        None | Some("") => (None, None),
        Some(lines) => parse_range(lines)?,
    };

    Ok(ParsedLocation {
        file_path: file_path.to_string(),
        start_line,
        end_line,
    })
}

fn parse_range(raw: &str) -> anyhow::Result<(Option<i64>, Option<i64>)> {
    if let Some((start, end)) = raw.split_once('-') {
        let start = parse_positive_line(start.trim())?;
        let end = parse_positive_line(end.trim())?;
        if end < start {
            anyhow::bail!("line range end must be >= start");
        }
        Ok((Some(start), Some(end)))
    } else {
        Ok((Some(parse_positive_line(raw.trim())?), None))
    }
}

fn parse_positive_line(raw: &str) -> anyhow::Result<i64> {
    let value: i64 = raw
        .parse()
        .map_err(|error| anyhow::anyhow!("invalid line '{}': {}", raw, error))?;
    if value <= 0 {
        anyhow::bail!("line numbers must be positive");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::parse_location;

    #[test]
    fn parses_plain_path() {
        let parsed = parse_location("src/main.rs").expect("location should parse");
        assert_eq!(parsed.file_path, "src/main.rs");
        assert_eq!(parsed.start_line, None);
        assert_eq!(parsed.end_line, None);
    }

    #[test]
    fn parses_single_line() {
        let parsed = parse_location("src/main.rs#10").expect("location should parse");
        assert_eq!(parsed.start_line, Some(10));
        assert_eq!(parsed.end_line, None);
    }

    #[test]
    fn parses_line_range() {
        let parsed = parse_location("src/main.rs#10-20").expect("location should parse");
        assert_eq!(parsed.start_line, Some(10));
        assert_eq!(parsed.end_line, Some(20));
    }

    #[test]
    fn rejects_inverted_range() {
        let err = parse_location("src/main.rs#20-10").expect_err("should fail");
        assert!(err.to_string().contains("end must be >= start"));
    }
}
