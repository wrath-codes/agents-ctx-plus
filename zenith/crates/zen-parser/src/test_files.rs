//! Test file and directory detection for filtering during indexing.
//!
//! Used by the walker factory (`zen-search/src/walk.rs`) to skip test
//! files and directories during package indexing. Patterns cover Go, Rust,
//! JavaScript/TypeScript, Python, Elixir, and common framework conventions.

/// Directory names conventionally used for tests, benchmarks, examples, and fixtures.
const TEST_DIRS: &[&str] = &[
    "test",
    "tests",
    "spec",
    "specs",
    "__tests__",
    "__mocks__",
    "__snapshots__",
    "testdata",
    "test_data",
    "fixtures",
    "e2e",
    "integration_tests",
    "unit_tests",
    "benches",
    "benchmarks",
    "examples",
];

/// Returns `true` if `dir_name` matches a known test/fixture directory convention.
///
/// Comparison is case-sensitive (directory names are almost always lowercase).
///
/// # Examples
///
/// ```
/// use zen_parser::is_test_dir;
/// assert!(is_test_dir("tests"));
/// assert!(is_test_dir("__tests__"));
/// assert!(!is_test_dir("src"));
/// ```
#[must_use]
pub fn is_test_dir(dir_name: &str) -> bool {
    TEST_DIRS.contains(&dir_name)
}

/// Returns `true` if `file_name` matches a known test file naming convention.
///
/// Supports conventions for:
/// - **Go**: `*_test.go`
/// - **Rust**: `*_test.rs`
/// - **JavaScript/TypeScript**: `*.test.{js,ts,tsx,jsx}`, `*.spec.{js,ts,tsx,jsx}`
/// - **Python**: `test_*.py`, `*_test.py`, `conftest.py`
/// - **Elixir**: `*_test.exs`
/// - **Go setup**: `setup_test.go`
///
/// Comparison is case-insensitive for the file name.
///
/// # Examples
///
/// ```
/// use zen_parser::is_test_file;
/// assert!(is_test_file("widget_test.go"));
/// assert!(is_test_file("App.test.tsx"));
/// assert!(is_test_file("test_utils.py"));
/// assert!(!is_test_file("main.rs"));
/// ```
#[must_use]
pub fn is_test_file(file_name: &str) -> bool {
    let name = file_name.to_lowercase();

    // Go
    name.ends_with("_test.go")
    // Rust
    || name.ends_with("_test.rs")
    // JavaScript / TypeScript (.test.*)
    || name.ends_with(".test.js")
    || name.ends_with(".test.ts")
    || name.ends_with(".test.tsx")
    || name.ends_with(".test.jsx")
    || name.ends_with(".test.mjs")
    || name.ends_with(".test.cjs")
    // JavaScript / TypeScript (.spec.*)
    || name.ends_with(".spec.js")
    || name.ends_with(".spec.ts")
    || name.ends_with(".spec.tsx")
    || name.ends_with(".spec.jsx")
    || name.ends_with(".spec.mjs")
    || name.ends_with(".spec.cjs")
    // Python
    || name.starts_with("test_") && std::path::Path::new(&name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("py"))
    || name.ends_with("_test.py")
    // Elixir
    || name.ends_with("_test.exs")
    // Special files
    || name == "conftest.py"
    || name == "setup_test.go"
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_test_dir ──────────────────────────────────────────

    #[test]
    fn test_dir_matches_common_names() {
        for dir in TEST_DIRS {
            assert!(is_test_dir(dir), "expected is_test_dir({dir:?}) == true");
        }
    }

    #[test]
    fn test_dir_rejects_production_dirs() {
        let prod = [
            "src", "lib", "bin", "cmd", "pkg", "internal", "app", "dist", "build",
        ];
        for dir in prod {
            assert!(!is_test_dir(dir), "expected is_test_dir({dir:?}) == false");
        }
    }

    #[test]
    fn test_dir_is_case_sensitive() {
        // TEST_DIRS are lowercase; uppercase should not match
        assert!(!is_test_dir("Tests"));
        assert!(!is_test_dir("TESTS"));
        assert!(!is_test_dir("__Tests__"));
    }

    // ── is_test_file — Go ────────────────────────────────────

    #[test]
    fn test_file_go() {
        assert!(is_test_file("handler_test.go"));
        assert!(is_test_file("setup_test.go"));
        assert!(!is_test_file("handler.go"));
    }

    // ── is_test_file — Rust ──────────────────────────────────

    #[test]
    fn test_file_rust() {
        assert!(is_test_file("parser_test.rs"));
        assert!(!is_test_file("parser.rs"));
        assert!(!is_test_file("mod.rs"));
    }

    // ── is_test_file — JavaScript / TypeScript ───────────────

    #[test]
    fn test_file_js_ts_test() {
        assert!(is_test_file("App.test.js"));
        assert!(is_test_file("App.test.ts"));
        assert!(is_test_file("App.test.tsx"));
        assert!(is_test_file("App.test.jsx"));
        assert!(is_test_file("App.test.mjs"));
        assert!(is_test_file("App.test.cjs"));
    }

    #[test]
    fn test_file_js_ts_spec() {
        assert!(is_test_file("App.spec.js"));
        assert!(is_test_file("App.spec.ts"));
        assert!(is_test_file("App.spec.tsx"));
        assert!(is_test_file("App.spec.jsx"));
        assert!(is_test_file("App.spec.mjs"));
        assert!(is_test_file("App.spec.cjs"));
    }

    #[test]
    fn test_file_js_ts_rejects_production() {
        assert!(!is_test_file("App.tsx"));
        assert!(!is_test_file("index.ts"));
        assert!(!is_test_file("utils.js"));
    }

    // ── is_test_file — Python ────────────────────────────────

    #[test]
    fn test_file_python() {
        assert!(is_test_file("test_utils.py"));
        assert!(is_test_file("handler_test.py"));
        assert!(is_test_file("conftest.py"));
        assert!(!is_test_file("utils.py"));
        assert!(!is_test_file("main.py"));
    }

    // ── is_test_file — Elixir ────────────────────────────────

    #[test]
    fn test_file_elixir() {
        assert!(is_test_file("router_test.exs"));
        assert!(!is_test_file("router.ex"));
    }

    // ── is_test_file — case insensitivity ────────────────────

    #[test]
    fn test_file_case_insensitive() {
        assert!(is_test_file("Handler_Test.go"));
        assert!(is_test_file("APP.TEST.TSX"));
        assert!(is_test_file("TEST_UTILS.PY"));
    }

    // ── is_test_file — edge cases ────────────────────────────

    #[test]
    fn test_file_empty_and_dots() {
        assert!(!is_test_file(""));
        assert!(!is_test_file("."));
        assert!(!is_test_file(".."));
        assert!(!is_test_file(".test"));
    }
}
