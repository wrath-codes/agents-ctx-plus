/// Compute effective limit with precedence: local arg -> global flag -> fallback.
#[must_use]
pub fn effective_limit(local: Option<u32>, global: Option<u32>, fallback: u32) -> u32 {
    local.or(global).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::effective_limit;

    #[test]
    fn local_takes_precedence() {
        assert_eq!(effective_limit(Some(5), Some(10), 20), 5);
    }

    #[test]
    fn global_used_when_local_missing() {
        assert_eq!(effective_limit(None, Some(10), 20), 10);
    }

    #[test]
    fn fallback_used_when_none_set() {
        assert_eq!(effective_limit(None, None, 20), 20);
    }
}
