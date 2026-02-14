//! File walker factory for indexing and grep operations.
//!
//! Uses the `ignore` crate for gitignore-aware directory walking with support
//! for custom ignore files (.zenithignore) and override globs.
//!
//! ## Walking modes
//!
//! - `LocalProject`: respects `.gitignore`, skips `.zenith/`, supports `.zenithignore`
//!   and include/exclude globs. Used for `znt grep` on a developer's working tree.
//! - `Raw`: disables all standard filters. Walks every file, including hidden files
//!   and ignored directories. Used for indexing cloned repositories where we want
//!   complete coverage.

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use std::path::Path;

/// Walking mode for the file walker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkMode {
    /// Local project: respects .gitignore, skips .zenith/, supports .zenithignore.
    LocalProject,
    /// Raw: no filters. For indexing cloned repos where we want every file.
    Raw,
}

/// Build a file walker over `root` with the given mode and filters.
///
/// # Arguments
///
/// - `root`: The directory to walk.
/// - `mode`: Walk mode (`LocalProject` or `Raw`).
/// - `skip_tests`: When true, excludes test directories and files using
///   `zen_parser::is_test_dir()` and `zen_parser::is_test_file()`.
/// - `include_glob`: Optional glob pattern to whitelist files (e.g., `"*.rs"`).
/// - `exclude_glob`: Optional glob pattern to blacklist files/directories
///   (e.g., `"tests/"`). Prefix with `!` to negate when using override builder
///   directly — here you pass the raw pattern and we negate it.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use zen_search::walk::{build_walker, WalkMode};
///
/// let walker = build_walker(
///     Path::new("/path/to/project"),
///     WalkMode::LocalProject,
///     true,           // skip tests
///     None,           // no include glob
///     None,           // no exclude glob
/// );
/// ```
pub fn build_walker(
    root: &Path,
    mode: WalkMode,
    skip_tests: bool,
    include_glob: Option<&str>,
    exclude_glob: Option<&str>,
) -> ignore::Walk {
    let mut builder = WalkBuilder::new(root);

    match mode {
        WalkMode::LocalProject => {
            // Don't skip hidden files; .gitignore will still filter node_modules, etc.
            builder.hidden(false);
            builder.add_custom_ignore_filename(".zenithignore");

            // Always exclude .zenith/ regardless of .gitignore
            let mut overrides = OverrideBuilder::new(root);
            if let Some(glob) = include_glob {
                overrides.add(glob).expect("valid include glob");
            }
            if let Some(glob) = exclude_glob {
                // Caller passes raw pattern, we negate it to exclude
                overrides
                    .add(&format!("!{glob}"))
                    .expect("valid exclude glob");
            }
            builder.overrides(overrides.build().expect("valid overrides"));
        }
        WalkMode::Raw => {
            // Disable all default filters (gitignore, hidden, etc.)
            builder.standard_filters(false);
            builder.hidden(false);

            // Still support include/exclude globs in raw mode if provided
            if include_glob.is_some() || exclude_glob.is_some() {
                let mut overrides = OverrideBuilder::new(root);
                if let Some(glob) = include_glob {
                    overrides.add(glob).expect("valid include glob");
                }
                if let Some(glob) = exclude_glob {
                    overrides
                        .add(&format!("!{glob}"))
                        .expect("valid exclude glob");
                }
                builder.overrides(overrides.build().expect("valid overrides"));
            }
        }
    }

    // Always exclude .zenith/ in LocalProject mode, and optionally skip tests
    if matches!(mode, WalkMode::LocalProject) {
        builder.filter_entry(move |entry| {
            let file_name = entry.file_name().to_string_lossy();
            // Always exclude .zenith directory
            if file_name == ".zenith" && entry.file_type().is_some_and(|ft| ft.is_dir()) {
                return false;
            }
            if skip_tests {
                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    !zen_parser::is_test_dir(&file_name)
                } else {
                    !zen_parser::is_test_file(&file_name)
                }
            } else {
                true
            }
        });
    } else if skip_tests {
        // Raw mode with skip_tests
        builder.filter_entry(move |entry| {
            let file_name = entry.file_name().to_string_lossy();
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                !zen_parser::is_test_dir(&file_name)
            } else {
                !zen_parser::is_test_file(&file_name)
            }
        });
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Helper: create a fixture directory with various files and subdirs
    fn create_fixture(dir: &Path) {
        let dirs = [
            "src",
            "src/handlers",
            "tests",
            "tests/fixtures",
            "node_modules/lodash",
            ".zenith/db",
            "vendor/deps",
        ];
        for d in &dirs {
            fs::create_dir_all(dir.join(d)).expect("mkdir should succeed");
        }

        let files = [
            ("src/main.rs", "fn main() { println!(\"hello\"); }"),
            ("src/lib.rs", "pub mod handlers;"),
            ("src/handlers/api.rs", "pub fn handle() {}"),
            ("tests/integration.rs", "#[test] fn it_works() {}"),
            ("tests/fixtures/data.json", "{\"key\": \"value\"}"),
            ("node_modules/lodash/index.js", "module.exports = {};"),
            (".zenith/db/main.db", "binary-data"),
            ("vendor/deps/dep.rs", "fn vendored() {}"),
            (".gitignore", "node_modules/\nvendor/\n"),
            ("Cargo.toml", "[package]\nname = \"test\""),
            ("README.md", "# Test Project"),
            (".hidden_file", "secret"),
        ];
        for (path, content) in &files {
            fs::write(dir.join(path), content).expect("write should succeed");
        }
    }

    #[test]
    fn raw_mode_walks_all_files() {
        let tmp = tempfile::tempdir().unwrap();
        create_fixture(tmp.path());

        let walker = build_walker(tmp.path(), WalkMode::Raw, false, None, None);
        let entries: Vec<String> = walker
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .map(|e| {
                e.path()
                    .strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // .gitignore doesn't apply in Raw mode
        assert!(entries.contains(&"node_modules/lodash/index.js".to_string()));
        assert!(entries.contains(&"vendor/deps/dep.rs".to_string()));
        assert!(entries.contains(&".hidden_file".to_string()));
        assert!(entries.contains(&"src/main.rs".to_string()));
        assert!(entries.contains(&"tests/integration.rs".to_string()));
    }

    #[test]
    fn local_project_respects_gitignore() {
        let tmp = tempfile::tempdir().unwrap();
        create_fixture(tmp.path());

        // Initialize a git repo so .gitignore is effective
        let _ = std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(tmp.path())
            .status()
            .expect("git init should succeed");

        let walker = build_walker(tmp.path(), WalkMode::LocalProject, false, None, None);
        let entries: Vec<String> = walker
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .map(|e| {
                e.path()
                    .strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // node_modules/ and vendor/ should be excluded by .gitignore
        assert!(!entries.iter().any(|e| e.contains("node_modules")));
        assert!(!entries.iter().any(|e| e.contains("vendor")));

        // Regular source files should be present
        assert!(entries.contains(&"src/main.rs".to_string()));
        assert!(entries.contains(&"README.md".to_string()));
    }

    #[test]
    fn skip_tests_excludes_test_dirs_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        create_fixture(tmp.path());

        let walker = build_walker(tmp.path(), WalkMode::Raw, true, None, None);
        let entries: Vec<String> = walker
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .map(|e| {
                e.path()
                    .strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // tests/ directory should be entirely excluded
        assert!(!entries.iter().any(|e| e.starts_with("tests")));
        // test files with pattern _test.rs, .test.ts, .spec.ts should be excluded
        // (none in fixture, but good to verify the filter doesn't crash)
        assert!(entries.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn zenithignore_custom_ignore() {
        let tmp = tempfile::tempdir().unwrap();
        create_fixture(tmp.path());

        fs::write(tmp.path().join(".zenithignore"), "README.md\nvendor/\n")
            .expect("write .zenithignore");

        let walker = build_walker(tmp.path(), WalkMode::LocalProject, false, None, None);
        let entries: Vec<String> = walker
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .map(|e| {
                e.path()
                    .strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert!(!entries.contains(&"README.md".to_string()));
        assert!(!entries.iter().any(|e| e.contains("vendor")));
        // .zenith/ directory is always excluded in LocalProject mode (but not .zenithignore file)
        assert!(
            !entries.iter().any(|e| e.starts_with(".zenith/")),
            ".zenith/ directory entries found: {:?}",
            entries
                .iter()
                .filter(|e| e.starts_with(".zenith/"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn include_exclude_globs() {
        let tmp = tempfile::tempdir().unwrap();
        create_fixture(tmp.path());

        // Only .rs files, exclude tests/ and .zenith/
        let walker = build_walker(
            tmp.path(),
            WalkMode::LocalProject,
            false,
            Some("*.rs"),
            Some("tests/"),
        );
        let entries: Vec<String> = walker
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .map(|e| {
                e.path()
                    .strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert!(entries.iter().all(|e| e.ends_with(".rs")));
        assert!(!entries.iter().any(|e| e.starts_with("tests")));
        assert!(!entries.iter().any(|e| e.starts_with(".zenith")));
        assert!(entries.contains(&"src/main.rs".to_string()));
        assert!(entries.contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn zenith_dir_always_excluded() {
        let tmp = tempfile::tempdir().unwrap();
        create_fixture(tmp.path());

        // Even with Raw mode and no excludes, .zenith/ should only be excluded
        // when using LocalProject mode. In Raw mode, it's not special.
        let walker = build_walker(tmp.path(), WalkMode::Raw, false, None, None);
        let entries: Vec<String> = walker
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .map(|e| {
                e.path()
                    .strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // In Raw mode, .zenith/ is NOT automatically excluded
        assert!(entries.iter().any(|e| e.starts_with(".zenith")));
    }
}
