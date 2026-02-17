use serde::de::DeserializeOwned;

/// Parse a snake_case enum value using serde-deserialization.
pub fn parse_enum<T>(raw: &str, field: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let normalized = raw.replace('-', "_");
    let json = format!("\"{normalized}\"");
    serde_json::from_str(&json).map_err(|error| anyhow::anyhow!("invalid {field} '{raw}': {error}"))
}

#[cfg(test)]
mod tests {
    use zen_core::enums::{StudyMethodology, StudyStatus};

    use super::parse_enum;

    #[test]
    fn parses_snake_case_enum() {
        let status: StudyStatus = parse_enum("completed", "status").expect("status should parse");
        assert_eq!(status, StudyStatus::Completed);
    }

    #[test]
    fn parses_hyphenated_alias() {
        let methodology: StudyMethodology =
            parse_enum("test-driven", "methodology").expect("methodology should parse");
        assert_eq!(methodology, StudyMethodology::TestDriven);
    }

    #[test]
    fn errors_on_invalid_enum() {
        let err = parse_enum::<StudyStatus>("done", "status").expect_err("should fail");
        assert!(err.to_string().contains("invalid status 'done'"));
    }
}
