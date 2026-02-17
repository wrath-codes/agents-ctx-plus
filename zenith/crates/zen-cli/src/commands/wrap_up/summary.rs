use crate::cli::root_commands::WrapUpArgs;

pub fn resolve_summary(args: &WrapUpArgs) -> String {
    args.summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .unwrap_or("Session wrapped up")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::resolve_summary;
    use crate::cli::root_commands::WrapUpArgs;

    #[test]
    fn uses_explicit_summary() {
        let args = WrapUpArgs {
            summary: Some("  done for today ".to_string()),
            auto_commit: false,
            message: None,
        };
        assert_eq!(resolve_summary(&args), "done for today");
    }

    #[test]
    fn falls_back_to_default_summary() {
        let args = WrapUpArgs {
            summary: None,
            auto_commit: false,
            message: None,
        };
        assert_eq!(resolve_summary(&args), "Session wrapped up");
    }
}
