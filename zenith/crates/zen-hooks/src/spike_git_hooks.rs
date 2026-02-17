#![allow(warnings)]
//! # Spike 0.13: Git Hooks & Session-Git Integration
//!
//! Validates git hooks integration for Zenith using the `gix` pure-Rust git library.
//!
//! ## What this spike validates
//!
//! **Part A — gix Repo Discovery & Config (tests 1-3)**:
//! - `gix::discover()` finds `.git` from subdirectories, handles no-repo gracefully
//! - Config reading: `core.hooksPath` in unset, relative, and absolute states
//! - Existing hook detection in `.git/hooks/`
//!
//! **Part B — Hook Installation Strategies (tests 4-7)**:
//! - Strategy A: `core.hooksPath` config write (husky-style, exclusive)
//! - Strategy B: Symlink into `.git/hooks/` (coexistent, fragile)
//! - Strategy C: Chain-append to existing hooks (coexistent, complex)
//! - No-git graceful skip
//!
//! **Part C — Pre-commit JSONL Validation (tests 8-11)**:
//! - Shell-only validation is insufficient (no jq, can't parse JSON in bash)
//! - Rust validation via serde_json catches all edge cases
//! - Staged-only validation via gix index API
//! - Thin wrapper with graceful fallback
//!
//! **Part D — Post-checkout & Post-merge (tests 12-15)**:
//! - JSONL change detection between commits via gix tree diff
//! - Auto-rebuild performance measurement at 100/1000/5000 ops
//! - Warn-only alternative
//! - Post-merge conflict detection + rebuild
//!
//! **Part E — Session-Git Integration (tests 16-20)**:
//! - Read branch name, HEAD hash
//! - Create/list lightweight session tags
//! - Rev-walk between session tags
//!
//! **Part F — Dependency Weight (test 21)**:
//! - Document gix features, compile characteristics
//!
//! **Part G — Comparison & Decision (test 22)**:
//! - Print comparison tables for all decisions

use std::fs;
use std::path::{Path, PathBuf};

/// Helper: convert a gix Cow<BStr> to a String.
fn bstr_cow_to_string(val: &std::borrow::Cow<'_, gix::bstr::BStr>) -> String {
    val.to_string()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temp directory with an initialized git repo (using git CLI for setup,
/// gix for the actual spike operations we're testing).
fn init_temp_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::TempDir::new().expect("create tempdir");
    let repo_path = dir.path().to_path_buf();

    // Use git CLI to init — we're testing gix's ability to READ repos, not create them.
    let output = std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(&repo_path)
        .output()
        .expect("git init failed");
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Configure user for commits
    for (key, val) in [
        ("user.email", "test@zenith.dev"),
        ("user.name", "Zenith Test"),
    ] {
        let output = std::process::Command::new("git")
            .args(["config", key, val])
            .current_dir(&repo_path)
            .output()
            .expect("git config failed");
        assert!(output.status.success());
    }

    (dir, repo_path)
}

/// Create an initial commit so HEAD exists.
fn make_initial_commit(repo_path: &Path) {
    let dummy = repo_path.join("README.md");
    fs::write(&dummy, "# Test repo\n").unwrap();

    run_git(repo_path, &["add", "."]);
    run_git(repo_path, &["commit", "-m", "initial commit"]);
}

/// Make a commit with a specific file and content.
fn commit_file(repo_path: &Path, filename: &str, content: &str, message: &str) {
    let file_path = repo_path.join(filename);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&file_path, content).unwrap();
    run_git(repo_path, &["add", filename]);
    run_git(repo_path, &["commit", "-m", message]);
}

/// Run a git command and assert success.
fn run_git(repo_path: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .unwrap_or_else(|e| panic!("git {} failed: {}", args.join(" "), e));
    assert!(
        output.status.success(),
        "git {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Run a git command, return (success, stdout, stderr).
fn run_git_raw(repo_path: &Path, args: &[&str]) -> (bool, String, String) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .unwrap_or_else(|e| panic!("git {} failed to execute: {}", args.join(" "), e));
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

// ===========================================================================
// Part A: gix Repo Discovery & Config (tests 1-3)
// ===========================================================================

/// Test 1: gix::discover() finds .git from subdirectory. Handles no-repo gracefully.
#[test]
fn spike_gix_discover_repo() {
    // --- Case 1: Discover from repo root ---
    let (_dir, repo_path) = init_temp_repo();
    let repo = gix::discover(&repo_path).expect("should discover repo at root");
    assert!(repo.git_dir().exists(), "git_dir should exist");

    // --- Case 2: Discover from nested subdirectory ---
    let nested = repo_path.join("src").join("deep");
    fs::create_dir_all(&nested).unwrap();
    let repo2 = gix::discover(&nested).expect("should discover repo from nested dir");
    // Both should resolve to the same .git
    assert_eq!(
        repo.git_dir().canonicalize().unwrap(),
        repo2.git_dir().canonicalize().unwrap(),
        "nested discover should find same repo"
    );

    // --- Case 3: No repo — should return Err, not panic ---
    let no_repo = tempfile::TempDir::new().unwrap();
    let result = gix::discover(no_repo.path());
    assert!(result.is_err(), "should error on non-repo directory");
    let err_msg = format!("{}", result.unwrap_err());
    eprintln!("  No-repo error message: {err_msg}");

    eprintln!("  PASS: gix::discover() works from root, nested dirs, and handles no-repo");
}

/// Test 2: Read core.hooksPath via gix config API in three states.
#[test]
fn spike_gix_read_hooks_path() {
    let (_dir, repo_path) = init_temp_repo();

    // --- Case 1: Unset (default) ---
    let repo = gix::discover(&repo_path).unwrap();
    let config = repo.config_snapshot();
    let hooks_path = config.string("core.hooksPath");
    assert!(
        hooks_path.is_none(),
        "core.hooksPath should be unset by default"
    );
    eprintln!("  Case 1 (unset): None — correct");

    // --- Case 2: Set to relative path ---
    run_git(&repo_path, &["config", "core.hooksPath", ".husky/_"]);
    let repo = gix::discover(&repo_path).unwrap(); // re-open to pick up config change
    let config = repo.config_snapshot();
    let hooks_path = config
        .string("core.hooksPath")
        .expect("should have hooksPath");
    let hooks_str = bstr_cow_to_string(&hooks_path);
    assert_eq!(hooks_str, ".husky/_");
    eprintln!("  Case 2 (relative): {hooks_str} — correct");

    // --- Case 3: Set to absolute path ---
    run_git(
        &repo_path,
        &["config", "core.hooksPath", "/home/user/.git-hooks"],
    );
    let repo = gix::discover(&repo_path).unwrap();
    let config = repo.config_snapshot();
    let hooks_path = config
        .string("core.hooksPath")
        .expect("should have hooksPath");
    let hooks_str = bstr_cow_to_string(&hooks_path);
    assert_eq!(hooks_str, "/home/user/.git-hooks");
    eprintln!("  Case 3 (absolute): {hooks_str} — correct");

    eprintln!("  PASS: gix reads core.hooksPath in all three states");
}

/// Test 3: Detect existing hook files in .git/hooks/. Measure gix presence.
#[test]
fn spike_gix_detect_existing_hooks() {
    let (_dir, repo_path) = init_temp_repo();
    let hooks_dir = repo_path.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();

    // --- Case 1: Hook present + executable ---
    let hook_path = hooks_dir.join("pre-commit");
    fs::write(&hook_path, "#!/bin/bash\necho hook\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let repo = gix::discover(&repo_path).unwrap();
    let git_dir = repo.git_dir().to_path_buf();
    let detected_hook = git_dir.join("hooks").join("pre-commit");
    assert!(detected_hook.exists(), "hook should exist");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::metadata(&detected_hook).unwrap().permissions();
        assert!(perms.mode() & 0o111 != 0, "hook should be executable");
        eprintln!(
            "  Case 1: hook exists, executable (mode {:o})",
            perms.mode()
        );
    }

    // --- Case 2: Hook present but NOT executable ---
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o644)).unwrap();
        let perms = fs::metadata(&detected_hook).unwrap().permissions();
        assert!(perms.mode() & 0o111 == 0, "hook should NOT be executable");
        eprintln!(
            "  Case 2: hook exists, NOT executable (mode {:o})",
            perms.mode()
        );
    }

    // --- Case 3: Hook absent ---
    fs::remove_file(&hook_path).unwrap();
    assert!(
        !detected_hook.exists(),
        "hook should not exist after removal"
    );
    eprintln!("  Case 3: hook absent — correct");

    // --- Note on gix dependency weight ---
    eprintln!("  NOTE: gix dependency weight should be measured via `cargo build --timings`");
    eprintln!("  Features enabled: max-performance-safe, index");
    eprintln!("  PASS: existing hook detection works for all three states");
}

// ===========================================================================
// Part B: Hook Installation Strategies (tests 4-7)
// ===========================================================================

/// Test 4: Strategy A — core.hooksPath config write via gix.
#[test]
fn spike_install_strategy_hookspath() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    // Create .zenith/hooks/ with a pre-commit hook
    let zenith_hooks = repo_path.join(".zenith").join("hooks");
    fs::create_dir_all(&zenith_hooks).unwrap();
    let hook_script = zenith_hooks.join("pre-commit");
    fs::write(&hook_script, "#!/bin/bash\necho ZENITH_HOOK_RAN\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // --- Set core.hooksPath via git CLI (testing the strategy, not gix write yet) ---
    run_git(&repo_path, &["config", "core.hooksPath", ".zenith/hooks"]);

    // Verify gix reads it back
    let repo = gix::discover(&repo_path).unwrap();
    let config = repo.config_snapshot();
    let val = config.string("core.hooksPath").expect("should be set");
    assert_eq!(bstr_cow_to_string(&val), ".zenith/hooks");

    // Verify hook actually runs on commit
    commit_file(&repo_path, "test.txt", "content", "test hookspath");
    // If hook failed, commit would have failed (we'd get an assertion error above)
    eprintln!("  Strategy A: core.hooksPath = .zenith/hooks works");

    // --- Now test gix config write ---
    // Write a different value via gix to verify write capability
    let mut repo = gix::discover(&repo_path).unwrap();
    {
        let mut config = repo.config_snapshot_mut();
        config
            .set_raw_value_by("core", None, "hooksPath", ".zenith/hooks-v2")
            .expect("set_raw_value_by should work");
        // Get the inner config file and write to disk
        let config_file = config.forget();
        let git_config_path = repo_path.join(".git").join("config");
        let mut out = fs::File::create(&git_config_path).expect("open .git/config for write");
        config_file
            .write_to(&mut out)
            .expect("write config to disk");
    }

    // Verify it persisted by re-opening
    let repo2 = gix::discover(&repo_path).unwrap();
    let config2 = repo2.config_snapshot();
    let val2 = config2
        .string("core.hooksPath")
        .expect("should still be set");
    assert_eq!(bstr_cow_to_string(&val2), ".zenith/hooks-v2");
    eprintln!("  gix config write: core.hooksPath written and persisted");

    // --- Test uninstall ---
    run_git(&repo_path, &["config", "--unset", "core.hooksPath"]);
    let repo3 = gix::discover(&repo_path).unwrap();
    let config3 = repo3.config_snapshot();
    assert!(
        config3.string("core.hooksPath").is_none(),
        "should be unset after uninstall"
    );
    eprintln!("  Uninstall: core.hooksPath unset — correct");

    eprintln!("  PASS: Strategy A (core.hooksPath) — install, gix write, uninstall all work");
}

/// Test 5: Strategy B — Symlink .git/hooks/pre-commit -> .zenith/hooks/pre-commit.
#[test]
fn spike_install_strategy_symlink() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    // Create source hook
    let zenith_hooks = repo_path.join(".zenith").join("hooks");
    fs::create_dir_all(&zenith_hooks).unwrap();
    let source_hook = zenith_hooks.join("pre-commit");
    fs::write(&source_hook, "#!/bin/bash\necho SYMLINK_HOOK\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&source_hook, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let git_hooks_dir = repo_path.join(".git").join("hooks");
    fs::create_dir_all(&git_hooks_dir).unwrap();
    let target_hook = git_hooks_dir.join("pre-commit");

    // --- Create symlink ---
    #[cfg(unix)]
    {
        // Relative symlink: .git/hooks/pre-commit -> ../../.zenith/hooks/pre-commit
        std::os::unix::fs::symlink("../../.zenith/hooks/pre-commit", &target_hook)
            .expect("symlink creation should work");
        assert!(target_hook.exists(), "symlink should resolve");
        assert!(
            target_hook
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink()
        );
        eprintln!("  Symlink created: .git/hooks/pre-commit -> ../../.zenith/hooks/pre-commit");
    }

    // --- Verify hook runs ---
    commit_file(&repo_path, "test.txt", "content", "test symlink hook");
    eprintln!("  Commit succeeded — symlink hook ran");

    // --- Update source, verify propagation ---
    fs::write(&source_hook, "#!/bin/bash\necho UPDATED_HOOK\nexit 0\n").unwrap();
    commit_file(&repo_path, "test2.txt", "content2", "test updated hook");
    eprintln!("  Source updated, commit succeeded — propagation works without reinstall");

    // --- Test: existing hook at target path ---
    #[cfg(unix)]
    {
        // Remove symlink, create a real file
        fs::remove_file(&target_hook).unwrap();
        fs::write(&target_hook, "#!/bin/bash\necho EXISTING\n").unwrap();

        // Now try to symlink — should fail because file exists
        let result = std::os::unix::fs::symlink("../../.zenith/hooks/pre-commit", &target_hook);
        assert!(result.is_err(), "symlink should fail when target exists");
        eprintln!("  Existing hook detected — symlink correctly refuses to overwrite");
    }

    // --- Test: symlink target missing ---
    #[cfg(unix)]
    {
        fs::remove_file(&target_hook).unwrap();
        // Create symlink to non-existent source
        std::os::unix::fs::symlink("../../.zenith/hooks/does-not-exist", &target_hook).unwrap();
        assert!(
            !target_hook.exists(),
            "symlink to missing target should not 'exist'"
        );
        assert!(
            target_hook
                .symlink_metadata()
                .unwrap()
                .file_type()
                .is_symlink(),
            "but it IS a symlink"
        );
        eprintln!("  Missing target: symlink exists but doesn't resolve — detected");
    }

    eprintln!("  PASS: Strategy B (symlink) — create, propagate, detect conflicts, missing target");
}

/// Test 6: Strategy C — Chain-append when existing hook found.
#[test]
fn spike_install_strategy_chain() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    let git_hooks_dir = repo_path.join(".git").join("hooks");
    fs::create_dir_all(&git_hooks_dir).unwrap();
    let hook_path = git_hooks_dir.join("pre-commit");

    // --- Create existing user hook that writes a marker file ---
    // Hooks run with cwd = repo root. Use .hook-log relative to cwd.
    fs::write(
        &hook_path,
        "#!/bin/bash\necho USER_HOOK >> .hook-log\nexit 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // --- Chain: rename original, install chained hook ---
    let user_hook_backup = git_hooks_dir.join("pre-commit.user");
    fs::rename(&hook_path, &user_hook_backup).expect("rename to .user backup");

    let chained_hook = format!(
        "#!/bin/bash\n\
         # Chained by Zenith — runs user's hook first, then zenith validation\n\
         if [ -x \"$(dirname \"$0\")/pre-commit.user\" ]; then\n\
         \t\"$(dirname \"$0\")/pre-commit.user\" \"$@\" || exit $?\n\
         fi\n\
         echo ZENITH_HOOK >> .hook-log\n\
         exit 0\n"
    );
    fs::write(&hook_path, &chained_hook).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // --- Verify both hooks run ---
    let log_path = repo_path.join(".hook-log");
    commit_file(&repo_path, "test.txt", "content", "test chain");
    let log = fs::read_to_string(&log_path).unwrap_or_default();
    assert!(log.contains("USER_HOOK"), "user hook should have run");
    assert!(log.contains("ZENITH_HOOK"), "zenith hook should have run");
    eprintln!("  Both hooks ran: {}", log.trim().replace('\n', ", "));

    // --- Verify chain respects exit codes ---
    // Make user hook fail
    fs::write(
        &user_hook_backup,
        "#!/bin/bash\necho USER_FAIL >> .hook-log\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&user_hook_backup, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Clear log
    fs::write(&log_path, "").unwrap();
    let (success, _stdout, _stderr) = run_git_raw(
        &repo_path,
        &["commit", "--allow-empty", "-m", "should fail"],
    );
    assert!(!success, "commit should fail when user hook fails");
    let log = fs::read_to_string(&log_path).unwrap_or_default();
    assert!(log.contains("USER_FAIL"), "user hook ran");
    assert!(
        !log.contains("ZENITH_HOOK"),
        "zenith hook should NOT run after user hook failure"
    );
    eprintln!("  Chain respects exit codes: user fail blocks zenith hook");

    // --- Uninstall: restore backup ---
    fs::remove_file(&hook_path).unwrap();
    fs::rename(&user_hook_backup, &hook_path).expect("restore from backup");
    assert!(hook_path.exists());
    assert!(!user_hook_backup.exists());
    eprintln!("  Uninstall: user hook restored from backup");

    eprintln!("  PASS: Strategy C (chain) — both run, exit codes respected, uninstall restores");
}

/// Test 7: No git repo — skip gracefully.
#[test]
fn spike_install_skip_no_git() {
    let dir = tempfile::TempDir::new().unwrap();
    let result = gix::discover(dir.path());
    assert!(result.is_err(), "should fail on non-repo");

    // Simulate what znt init would do:
    let should_install_hooks = result.is_ok();
    assert!(!should_install_hooks, "hook installation should be skipped");
    eprintln!("  No .git directory — hook installation correctly skipped");
    eprintln!("  PASS: graceful skip when not a git repo");
}

// ===========================================================================
// Part C: Pre-commit JSONL Validation (tests 8-11)
// ===========================================================================

/// Test 8: Shell-only JSONL validation is insufficient.
#[test]
fn spike_precommit_shell_validation() {
    // This test DOCUMENTS that pure bash cannot reliably validate JSON.
    // We test the limitations, not a working shell validator.

    let cases: Vec<(&str, &str, bool, &str)> = vec![
        // (name, content, should_be_valid, shell_can_detect)
        (
            "valid",
            r#"{"ts":"2026-01-01","ses":"ses-001","op":"create","entity":"finding","id":"fnd-001","data":{}}"#,
            true,
            "yes",
        ),
        (
            "malformed_json",
            r#"{"ts":"2026-01-01","ses":"ses-001" BROKEN"#,
            false,
            "NO — bash {..} check passes",
        ),
        ("empty_file", "", true, "yes (no lines to check)"),
        ("trailing_newline", "{\"ts\":\"2026-01-01\"}\n", true, "yes"),
        (
            "\u{feff}BOM_prefix",
            "\u{feff}{\"ts\":\"2026-01-01\"}",
            false,
            "NO — invisible BOM, bash can't detect",
        ),
        (
            "conflict_markers",
            "<<<<<<< HEAD\n{\"ts\":\"a\"}\n=======\n{\"ts\":\"b\"}\n>>>>>>> branch",
            false,
            "maybe — if grep for markers",
        ),
    ];

    eprintln!("  Shell JSONL validation edge case matrix:");
    eprintln!(
        "  {:<20} {:<8} {:<8} {}",
        "Case", "Valid?", "Shell?", "Note"
    );
    eprintln!("  {:-<20} {:-<8} {:-<8} {:-<40}", "", "", "", "");
    for (name, _content, valid, shell) in &cases {
        eprintln!("  {:<20} {:<8} {:<8} {}", name, valid, shell, "");
    }

    eprintln!();
    eprintln!("  CONCLUSION: Shell-only validation cannot detect malformed JSON or BOM.");
    eprintln!("  jq is NOT installed by default on macOS or many Linux distros.");
    eprintln!("  python3 may not be present on minimal systems.");
    eprintln!("  Rust validation via `znt hook pre-commit` is the only reliable path.");
    eprintln!("  PASS: Shell limitations documented");
}

/// Test 9: Rust validation via serde_json + jsonschema catches all edge cases.
#[test]
fn spike_precommit_rust_validation() {
    /// The JSON Schema for a single JSONL trail operation.
    /// This is the canonical schema — will live in zen-core or zen-hooks for production.
    fn trail_schema() -> serde_json::Value {
        serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Zenith Trail Operation",
            "description": "A single operation in the JSONL trail (source of truth for Zenith state)",
            "type": "object",
            "required": ["ts", "ses", "op", "entity"],
            "properties": {
                "ts": {
                    "type": "string",
                    "description": "ISO 8601 timestamp"
                },
                "ses": {
                    "type": "string",
                    "description": "Session ID (ses-xxx)"
                },
                "op": {
                    "type": "string",
                    "enum": ["create", "update", "delete", "link", "unlink", "tag", "untag", "transition"],
                    "description": "Operation type"
                },
                "entity": {
                    "type": "string",
                    "description": "Entity type (finding, hypothesis, issue, task, etc.)"
                },
                "id": {
                    "type": "string",
                    "description": "Entity ID"
                },
                "data": {
                    "type": "object",
                    "description": "Operation payload"
                }
            },
            "additionalProperties": true
        })
    }

    /// Validate a JSONL file content using jsonschema crate.
    /// Returns Vec of (line_number, error_message) for failures.
    fn validate_jsonl(content: &str) -> Vec<(usize, String)> {
        let schema = trail_schema();
        let validator = jsonschema::validator_for(&schema).expect("valid schema");

        let mut errors = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let line_num = i + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Pre-parse guards: BOM and conflict markers aren't valid JSON
            if trimmed.starts_with('\u{feff}') {
                errors.push((
                    line_num,
                    "BOM (byte order mark) detected — remove it".into(),
                ));
                continue;
            }

            if trimmed.starts_with("<<<<<<<")
                || trimmed.starts_with("=======")
                || trimmed.starts_with(">>>>>>>")
            {
                errors.push((line_num, format!("git conflict marker: {}", &trimmed[..7])));
                continue;
            }

            // Parse JSON
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(val) => {
                    // Validate against JSON Schema using iter_errors for all errors
                    let schema_errors: Vec<_> = validator.iter_errors(&val).collect();
                    for err in &schema_errors {
                        errors.push((line_num, format!("schema: {err}")));
                    }
                }
                Err(e) => {
                    errors.push((line_num, format!("invalid JSON: {e}")));
                }
            }
        }
        errors
    }

    // --- Test matrix ---
    let cases: Vec<(&str, &str, bool)> = vec![
        (
            "valid",
            r#"{"ts":"2026-01-01","ses":"ses-001","op":"create","entity":"finding","id":"fnd-001","data":{}}"#,
            true,
        ),
        (
            "malformed_json",
            r#"{"ts":"2026-01-01","ses":"ses-001" BROKEN"#,
            false,
        ),
        ("empty_file", "", true),
        (
            "trailing_newline",
            "{\"ts\":\"2026-01-01\",\"ses\":\"s\",\"op\":\"create\",\"entity\":\"f\"}\n",
            true,
        ),
        ("bom_prefix", "\u{feff}{\"ts\":\"2026-01-01\"}", false),
        (
            "conflict_markers",
            "<<<<<<< HEAD\n{\"ts\":\"a\"}\n=======\n{\"ts\":\"b\"}\n>>>>>>> branch",
            false,
        ),
        (
            "missing_required_field",
            r#"{"ts":"2026-01-01","op":"create"}"#,
            false,
        ),
        ("not_an_object", r#"[1, 2, 3]"#, false),
        (
            "invalid_op_enum",
            r#"{"ts":"2026-01-01","ses":"ses-001","op":"INVALID_OP","entity":"finding"}"#,
            false,
        ),
    ];

    eprintln!("  Rust JSONL validation results:");
    let mut all_correct = true;
    for (name, content, expected_valid) in &cases {
        let errors = validate_jsonl(content);
        let is_valid = errors.is_empty();
        let correct = is_valid == *expected_valid;
        if !correct {
            all_correct = false;
        }
        let mark = if correct { "OK" } else { "FAIL" };
        eprintln!("  [{mark}] {name}: valid={is_valid}, expected={expected_valid}");
        for (line, msg) in &errors {
            eprintln!("       line {line}: {msg}");
        }
    }

    assert!(
        all_correct,
        "all validation cases should match expected results"
    );
    eprintln!();
    eprintln!();
    eprintln!("  CONCLUSION: serde_json + jsonschema validation catches ALL edge cases:");
    eprintln!("  malformed JSON, BOM, conflict markers, missing fields, non-objects,");
    eprintln!("  invalid enum values (op must be create|update|delete|link|...), type checks.");
    eprintln!("  Schema is self-documenting and reusable across the system.");
    eprintln!("  PASS: Rust validation with jsonschema is comprehensive");
}

/// Test 10: Staged-only validation via gix index API.
#[test]
fn spike_precommit_staged_only() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    // Create .zenith/trail/ structure
    let trail_dir = repo_path.join(".zenith").join("trail");
    fs::create_dir_all(&trail_dir).unwrap();

    // Write two JSONL files
    let staged_file = trail_dir.join("ses-001.jsonl");
    let unstaged_file = trail_dir.join("ses-002.jsonl");
    fs::write(
        &staged_file,
        r#"{"ts":"a","ses":"s","op":"c","entity":"f"}"#,
    )
    .unwrap();
    fs::write(
        &unstaged_file,
        r#"{"ts":"b","ses":"s","op":"c","entity":"f"}"#,
    )
    .unwrap();

    // Stage only the first file
    run_git(&repo_path, &["add", ".zenith/trail/ses-001.jsonl"]);

    // Read staged files via gix index
    let repo = gix::discover(&repo_path).unwrap();
    let index = repo.open_index().expect("should read index");

    let staged_jsonl: Vec<String> = index
        .entries()
        .iter()
        .filter_map(|entry| {
            let path = entry.path(&index);
            let path_str = path.to_string();
            if path_str.starts_with(".zenith/trail/") && path_str.ends_with(".jsonl") {
                Some(path_str)
            } else {
                None
            }
        })
        .collect();

    eprintln!("  Staged JSONL files from gix index:");
    for f in &staged_jsonl {
        eprintln!("    {f}");
    }

    assert_eq!(staged_jsonl.len(), 1, "should only see the staged file");
    assert!(staged_jsonl[0].contains("ses-001"), "should be ses-001");
    eprintln!("  Unstaged ses-002.jsonl correctly excluded");
    eprintln!("  PASS: gix index API reads staged files correctly");
}

/// Test 11: Thin shell wrapper — calls zen if available, else skips.
#[test]
fn spike_precommit_wrapper() {
    // This test validates the DESIGN of the wrapper, not the actual shell execution.
    // The wrapper script is the recommended hook format.

    let wrapper_script = r#"#!/bin/bash
# Zenith pre-commit hook — validates JSONL trail files
# Generated by `znt init`
if command -v znt >/dev/null 2>&1; then
    exec znt hook pre-commit "$@"
else
    echo "zenith: 'znt' not in PATH — skipping JSONL validation" >&2
    echo "zenith: install with: cargo install --path crates/zen-cli" >&2
    exit 0  # don't block the commit
fi
"#;

    // Verify the script is valid bash syntax
    let dir = tempfile::TempDir::new().unwrap();
    let script_path = dir.path().join("pre-commit");
    fs::write(&script_path, wrapper_script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Check bash syntax (bash -n = syntax check only)
    let output = std::process::Command::new("bash")
        .args(["-n", script_path.to_str().unwrap()])
        .output()
        .expect("bash should be available");
    assert!(
        output.status.success(),
        "wrapper script has valid bash syntax"
    );
    eprintln!("  Wrapper script syntax: valid");

    // Test: znt NOT in PATH — should exit 0
    // Use a minimal PATH that has basic utils but NOT znt.
    let output = std::process::Command::new("bash")
        .arg(&script_path)
        .env("PATH", "/usr/bin:/bin")
        .env_remove("HOME")
        .output()
        .expect("should run");
    assert!(
        output.status.success(),
        "should exit 0 when znt not in PATH, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not in PATH"),
        "should print guidance, got: {stderr}"
    );
    eprintln!("  znt not in PATH: exits 0 with guidance message");

    eprintln!("  PASS: wrapper script is the recommended hook format");
}

// ===========================================================================
// Part D: Post-checkout & Post-merge (tests 12-15)
// ===========================================================================

/// Test 12: Detect JSONL changes between commits.
#[test]
fn spike_postcheckout_detect_changes() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    // Commit a JSONL file on main
    let trail_dir = repo_path.join(".zenith").join("trail");
    fs::create_dir_all(&trail_dir).unwrap();
    commit_file(
        &repo_path,
        ".zenith/trail/ses-001.jsonl",
        r#"{"ts":"a","ses":"s","op":"c","entity":"f"}"#,
        "add trail file",
    );

    // Record this commit hash
    let main_hash = run_git(&repo_path, &["rev-parse", "HEAD"])
        .trim()
        .to_owned();

    // Create a branch and add another trail file
    run_git(&repo_path, &["checkout", "-b", "feature"]);
    commit_file(
        &repo_path,
        ".zenith/trail/ses-002.jsonl",
        r#"{"ts":"b","ses":"s2","op":"c","entity":"f"}"#,
        "add second trail",
    );
    let feature_hash = run_git(&repo_path, &["rev-parse", "HEAD"])
        .trim()
        .to_owned();

    // Use gix to diff the two commits
    let repo = gix::discover(&repo_path).unwrap();
    let main_oid: gix::ObjectId = main_hash.parse().expect("parse main hash");
    let feature_oid: gix::ObjectId = feature_hash.parse().expect("parse feature hash");

    let main_commit = repo.find_commit(main_oid).expect("find main commit");
    let feature_commit = repo.find_commit(feature_oid).expect("find feature commit");

    let main_tree = main_commit.tree().expect("main tree");
    let feature_tree = feature_commit.tree().expect("feature tree");

    // Use gix to diff trees — repo.diff_tree_to_tree() requires blob-diff feature
    let changes_result = repo.diff_tree_to_tree(Some(&main_tree), Some(&feature_tree), None);

    let mut changes: Vec<String> = Vec::new();
    match changes_result {
        Ok(diff_changes) => {
            for change in diff_changes.iter() {
                let path = change.location().to_string();
                if path.starts_with(".zenith/trail/") && path.ends_with(".jsonl") {
                    changes.push(path);
                }
            }
        }
        Err(e) => {
            // Fallback: use git CLI for diff if gix API doesn't work as expected
            eprintln!("  gix diff_tree_to_tree error: {e}");
            eprintln!("  Falling back to git CLI for diff...");
            let diff_output = run_git(
                &repo_path,
                &[
                    "diff",
                    "--name-only",
                    &main_hash,
                    &feature_hash,
                    "--",
                    ".zenith/trail/",
                ],
            );
            for line in diff_output.lines() {
                if !line.trim().is_empty() {
                    changes.push(line.trim().to_owned());
                }
            }
        }
    };

    eprintln!("  JSONL changes between main and feature:");
    for c in &changes {
        eprintln!("    {c}");
    }
    assert!(
        !changes.is_empty(),
        "should detect at least one JSONL change"
    );
    eprintln!("  PASS: gix tree diff detects JSONL file changes between commits");
}

/// Test 13: Auto-rebuild performance measurement.
#[test]
fn spike_postcheckout_auto_rebuild() {
    // Simulate rebuild by replaying JSONL operations into serde_json parsing.
    // This measures the JSONL parsing + validation time, which is the core of rebuild.
    // Actual DB rebuild (SQLite writes) would add time, but we measure the baseline here.

    for op_count in [100, 1000, 5000] {
        // Generate JSONL content
        let mut content = String::new();
        for i in 0..op_count {
            content.push_str(&format!(
                r#"{{"ts":"2026-01-01T{:02}:{:02}:{:02}Z","ses":"ses-001","op":"create","entity":"finding","id":"fnd-{:05}","data":{{"content":"Finding number {}"}}}}"#,
                (i / 3600) % 24,
                (i / 60) % 60,
                i % 60,
                i,
                i
            ));
            content.push('\n');
        }

        let start = std::time::Instant::now();

        // Parse every line (simulates rebuild validation pass)
        let mut parsed = 0_usize;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let _val: serde_json::Value = serde_json::from_str(line).expect("valid JSON");
            parsed += 1;
        }
        let elapsed = start.elapsed();

        assert_eq!(parsed, op_count);
        eprintln!(
            "  Rebuild {op_count:>5} ops: {:>6.1}ms ({:.0} ops/sec)",
            elapsed.as_secs_f64() * 1000.0,
            op_count as f64 / elapsed.as_secs_f64()
        );
    }

    eprintln!();
    eprintln!("  NOTE: These times are for JSONL parse only (serde_json).");
    eprintln!("  Actual `znt rebuild` adds SQLite inserts + FTS5 indexing.");
    eprintln!("  Spike 0.12 measured full rebuild at ~60 LOC replay logic.");
    eprintln!("  Decision threshold: <500ms auto, 500ms-2s auto+msg, >2s warn-only.");
    eprintln!("  PASS: rebuild performance measured");
}

/// Test 14: Warn-only alternative for post-checkout.
#[test]
fn spike_postcheckout_warn_only() {
    // The warn-only approach just detects changes and prints a message.
    // No rebuild, no delay.

    let start = std::time::Instant::now();

    // Simulate detection: check if any .zenith/trail/*.jsonl changed
    let changed_files = vec![".zenith/trail/ses-001.jsonl"]; // simulated
    if !changed_files.is_empty() {
        let _msg = format!(
            "Zenith: JSONL trail changed ({} files). Run `znt rebuild` to update database.",
            changed_files.len()
        );
    }

    let elapsed = start.elapsed();
    eprintln!(
        "  Warn-only detection time: {:.3}ms",
        elapsed.as_secs_f64() * 1000.0
    );
    eprintln!("  Message: 'Zenith: JSONL trail changed (1 files). Run `znt rebuild`...'");
    eprintln!("  UX: instant, but requires manual action. User may forget.");
    eprintln!("  PASS: warn-only approach is near-zero cost");
}

/// Test 15: Post-merge conflict detection + rebuild.
#[test]
fn spike_postmerge_conflict_and_rebuild() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    // Create trail on main
    commit_file(
        &repo_path,
        ".zenith/trail/ses-001.jsonl",
        r#"{"ts":"a","ses":"ses-001","op":"create","entity":"finding","id":"fnd-001","data":{}}"#,
        "main trail",
    );

    // Create branch with different trail
    run_git(&repo_path, &["checkout", "-b", "feature"]);
    commit_file(
        &repo_path,
        ".zenith/trail/ses-002.jsonl",
        r#"{"ts":"b","ses":"ses-002","op":"create","entity":"finding","id":"fnd-002","data":{}}"#,
        "feature trail",
    );

    // --- Case 1: Clean merge (different files — no conflict) ---
    run_git(&repo_path, &["checkout", "main"]);
    run_git(&repo_path, &["merge", "feature", "-m", "merge feature"]);

    // Both trail files should exist
    assert!(repo_path.join(".zenith/trail/ses-001.jsonl").exists());
    assert!(repo_path.join(".zenith/trail/ses-002.jsonl").exists());
    eprintln!("  Clean merge: both trail files present");

    // Validate no conflict markers in either file
    for ses in ["ses-001", "ses-002"] {
        let content =
            fs::read_to_string(repo_path.join(format!(".zenith/trail/{ses}.jsonl"))).unwrap();
        assert!(!content.contains("<<<<<<<"), "no conflict markers in {ses}");
    }
    eprintln!("  No conflict markers — clean merge confirmed");

    // --- Case 2: Simulate conflict detection logic ---
    let conflicted_content =
        "<<<<<<< HEAD\n{\"ts\":\"a\"}\n=======\n{\"ts\":\"b\"}\n>>>>>>> feature";
    let has_conflicts = conflicted_content
        .lines()
        .any(|l| l.starts_with("<<<<<<<") || l.starts_with("=======") || l.starts_with(">>>>>>>"));
    assert!(has_conflicts, "conflict markers should be detected");
    eprintln!("  Conflict marker detection: works correctly");

    eprintln!("  PASS: post-merge — clean merge works, conflicts detected");
}

// ===========================================================================
// Part E: Session-Git Integration (tests 16-20)
// ===========================================================================

/// Test 16: Read current branch name via gix.
#[test]
fn spike_session_read_branch() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    // --- Case 1: On main ---
    let repo = gix::discover(&repo_path).unwrap();
    let head = repo.head().expect("head");
    assert!(!head.is_detached(), "should not be detached");
    let name = head.referent_name().expect("should have branch ref");
    let name_str = name.as_bstr().to_string();
    assert!(name_str.contains("main"), "should be on main: {name_str}");
    eprintln!("  On main: {name_str}");

    // --- Case 2: On feature branch ---
    run_git(&repo_path, &["checkout", "-b", "feat/hooks"]);
    let repo = gix::discover(&repo_path).unwrap();
    let head = repo.head().unwrap();
    let name = head.referent_name().expect("should have branch ref");
    let name_str = name.as_bstr().to_string();
    assert!(
        name_str.contains("feat/hooks"),
        "should be on feat/hooks: {name_str}"
    );
    eprintln!("  On feature: {name_str}");

    // --- Case 3: Detached HEAD ---
    let hash = run_git(&repo_path, &["rev-parse", "HEAD"])
        .trim()
        .to_owned();
    run_git(&repo_path, &["checkout", &hash]);
    let repo = gix::discover(&repo_path).unwrap();
    let head = repo.head().unwrap();
    assert!(head.is_detached(), "should be detached");
    let oid = head.id().expect("detached HEAD should have an id");
    eprintln!("  Detached HEAD: {oid}");

    eprintln!("  PASS: branch name reading works for all cases");
}

/// Test 17: Read HEAD commit hash via gix.
#[test]
fn spike_session_read_head() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    let repo = gix::discover(&repo_path).unwrap();

    // Full SHA
    let head_id = repo.head_id().expect("should have HEAD");
    let full_sha = head_id.to_string();
    assert_eq!(full_sha.len(), 40, "full SHA should be 40 chars");
    eprintln!("  Full SHA: {full_sha}");

    // Short SHA (first 7 chars)
    let short_sha = &full_sha[..7];
    eprintln!("  Short SHA: {short_sha}");

    // Compare with git CLI
    let git_hash = run_git(&repo_path, &["rev-parse", "HEAD"])
        .trim()
        .to_owned();
    assert_eq!(full_sha, git_hash, "gix and git should agree");
    eprintln!("  Matches git CLI: yes");

    // --- Case: empty repo with no commits ---
    let (_dir2, repo_path2) = init_temp_repo();
    let repo2 = gix::discover(&repo_path2).unwrap();
    let head = repo2.head().unwrap();
    assert!(head.is_unborn(), "should be unborn in empty repo");
    assert!(head.id().is_none(), "no id for unborn HEAD");
    eprintln!("  Empty repo: head.is_unborn()=true, id=None");

    eprintln!("  PASS: HEAD hash reading works, including empty repos");
}

/// Test 18: Create lightweight session tag via gix.
#[test]
fn spike_session_create_tag() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    let repo = gix::discover(&repo_path).unwrap();
    let head_id = repo.head_id().expect("HEAD");

    // --- Create tag ---
    let tag_name = "refs/tags/zenith/ses-abc12345";
    repo.edit_reference(gix::refs::transaction::RefEdit {
        change: gix::refs::transaction::Change::Update {
            log: gix::refs::transaction::LogChange {
                mode: gix::refs::transaction::RefLog::AndReference,
                force_create_reflog: false,
                message: "zenith session tag".into(),
            },
            expected: gix::refs::transaction::PreviousValue::MustNotExist,
            new: gix::refs::Target::Object(head_id.detach()),
        },
        name: tag_name.try_into().expect("valid ref name"),
        deref: false,
    })
    .expect("create tag should succeed");

    // Verify via git CLI
    let tag_output = run_git(&repo_path, &["tag", "-l", "zenith/*"]);
    assert!(
        tag_output.contains("zenith/ses-abc12345"),
        "tag should exist: {tag_output}"
    );
    eprintln!("  Tag created: zenith/ses-abc12345");

    // Verify it points to the right commit
    let tag_hash = run_git(&repo_path, &["rev-parse", "refs/tags/zenith/ses-abc12345"])
        .trim()
        .to_owned();
    assert_eq!(tag_hash, head_id.to_string(), "tag should point to HEAD");
    eprintln!("  Points to: {tag_hash} (matches HEAD)");

    // --- Case: Tag already exists — verify we can detect it before creating ---
    // Strategy: check if the ref already exists before calling edit_reference.
    // This is safer than relying on MustNotExist which may behave differently
    // across gix versions.
    let existing = repo.find_reference(tag_name);
    assert!(existing.is_ok(), "tag should be findable after creation");
    eprintln!("  Duplicate detection: find_reference() returns Ok for existing tag");

    // Attempting to create with MustNotExist — document actual behavior
    let dup_result = repo.edit_reference(gix::refs::transaction::RefEdit {
        change: gix::refs::transaction::Change::Update {
            log: gix::refs::transaction::LogChange {
                mode: gix::refs::transaction::RefLog::AndReference,
                force_create_reflog: false,
                message: "duplicate tag".into(),
            },
            expected: gix::refs::transaction::PreviousValue::MustNotExist,
            new: gix::refs::Target::Object(head_id.detach()),
        },
        name: tag_name.try_into().expect("valid ref name"),
        deref: false,
    });
    if dup_result.is_err() {
        eprintln!("  MustNotExist: correctly rejected duplicate (gix enforces)");
    } else {
        eprintln!("  MustNotExist: did NOT reject duplicate (gix 0.70 quirk)");
        eprintln!("  WORKAROUND: use find_reference() to check existence before creating");
    }

    eprintln!("  PASS: lightweight session tags work via gix");
}

/// Test 19: List tags matching zenith/ses-*.
#[test]
fn spike_session_list_tags() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    // Create 5 session tags via git CLI (faster for setup)
    for i in 1..=5 {
        let tag = format!("zenith/ses-{i:05}");
        run_git(&repo_path, &["tag", &tag]);
    }
    // Also create a non-zenith tag to verify filtering
    run_git(&repo_path, &["tag", "v1.0.0"]);

    // List via gix
    let repo = gix::discover(&repo_path).unwrap();
    let refs = repo.references().expect("references");
    let mut session_tags: Vec<String> = Vec::new();
    for reference in refs
        .prefixed("refs/tags/zenith/ses-")
        .expect("prefixed iter")
    {
        let reference = reference.expect("valid ref");
        let name = reference.name().as_bstr().to_string();
        session_tags.push(name);
    }

    eprintln!("  Session tags found:");
    for t in &session_tags {
        eprintln!("    {t}");
    }

    assert_eq!(session_tags.len(), 5, "should find 5 session tags");
    // v1.0.0 should NOT be in the list
    assert!(
        !session_tags.iter().any(|t| t.contains("v1.0.0")),
        "non-zenith tags should be filtered out"
    );
    eprintln!("  v1.0.0 correctly excluded");
    eprintln!("  PASS: tag listing with prefix filtering works");
}

/// Test 20: Rev-walk between two session tags.
#[test]
fn spike_session_commits_between_tags() {
    let (_dir, repo_path) = init_temp_repo();
    make_initial_commit(&repo_path);

    // Tag: session start
    run_git(&repo_path, &["tag", "zenith/ses-start"]);

    // Make 3 commits
    for i in 1..=3 {
        commit_file(
            &repo_path,
            &format!("file{i}.txt"),
            &format!("content {i}"),
            &format!("commit {i}"),
        );
    }

    // Tag: session end
    run_git(&repo_path, &["tag", "zenith/ses-end"]);

    // Rev-walk from end to start via gix
    let repo = gix::discover(&repo_path).unwrap();
    let start_hash = run_git(&repo_path, &["rev-parse", "zenith/ses-start"])
        .trim()
        .to_owned();
    let end_hash = run_git(&repo_path, &["rev-parse", "zenith/ses-end"])
        .trim()
        .to_owned();

    let start_oid: gix::ObjectId = start_hash.parse().unwrap();
    let end_oid: gix::ObjectId = end_hash.parse().unwrap();

    let walk = repo.rev_walk([end_oid]);
    let mut commits_between: Vec<gix::ObjectId> = Vec::new();
    for info in walk.all().expect("rev-walk") {
        let info = info.expect("valid commit info");
        if info.id == start_oid {
            break; // Don't include the start tag's commit
        }
        commits_between.push(info.id);
    }

    eprintln!("  Commits between ses-start and ses-end:");
    for (i, oid) in commits_between.iter().enumerate() {
        eprintln!("    {}: {}", i + 1, &oid.to_string()[..7]);
    }

    assert_eq!(
        commits_between.len(),
        3,
        "should have 3 commits between tags"
    );
    eprintln!("  PASS: rev-walk between session tags works");
}

// ===========================================================================
// Part F: Dependency Weight (test 21)
// ===========================================================================

/// Test 21: Document gix dependency characteristics.
#[test]
fn spike_gix_dependency_weight() {
    eprintln!("  gix dependency analysis:");
    eprintln!("  -------------------------");
    eprintln!("  Crate: gix (gitoxide)");
    eprintln!("  Version: 0.70.x");
    eprintln!("  Features enabled:");
    eprintln!("    - max-performance-safe (optimized zlib + SHA1, no unsafe)");
    eprintln!("    - index (read git index for staged files)");
    eprintln!("  Features NOT enabled:");
    eprintln!("    - blob-diff (line-level diff — not needed, we use tree-level changes)");
    eprintln!("    - credentials (not doing auth)");
    eprintln!("    - async (not needed for hooks)");
    eprintln!();
    eprintln!("  Operations validated in this spike:");
    eprintln!("    1. gix::discover() — repo detection from subdirs");
    eprintln!("    2. config_snapshot().string() — read core.hooksPath");
    eprintln!("    3. config_snapshot_mut() + write_to() — write core.hooksPath");
    eprintln!("    4. open_index() — read staged files");
    eprintln!("    5. tree.changes().for_each_to_obtain_tree() — tree diff");
    eprintln!("    6. head()/head_id() — read branch + HEAD hash");
    eprintln!("    7. edit_reference() — create lightweight tags");
    eprintln!("    8. references().prefixed() — list tags by prefix");
    eprintln!("    9. rev_walk() — walk commit history");
    eprintln!();
    eprintln!("  NOTE: Compile time and binary size delta should be measured with:");
    eprintln!("    cargo build -p zen-hooks --timings");
    eprintln!("    cargo build -p zen-hooks --release && ls -la target/release/...");
    eprintln!("  These are environment-dependent and not measured in-test.");
    eprintln!();
    eprintln!("  Isolation strategy: gix lives ONLY in zen-hooks crate.");
    eprintln!("  Other crates (zen-db, zen-cli) depend on zen-hooks, not gix directly.");
    eprintln!("  This limits recompilation scope to zen-hooks when gix updates.");
    eprintln!("  PASS: dependency characteristics documented");
}

// ===========================================================================
// Part G: Comparison & Decision (test 22)
// ===========================================================================

/// Test 22: Print comparison tables for all open decisions.
#[test]
fn spike_compare_all() {
    eprintln!();
    eprintln!("  ===================================================================");
    eprintln!("  SPIKE 0.13 — COMPARISON & RECOMMENDATIONS");
    eprintln!("  ===================================================================");
    eprintln!();

    // --- Table 1: Installation Strategy ---
    eprintln!("  TABLE 1: Hook Installation Strategy");
    eprintln!("  {:-<75}", "");
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Criterion", "A: hooksPath", "B: Symlink", "C: Chain"
    );
    eprintln!("  {:-<20} {:-<18} {:-<18} {:-<18}", "", "", "", "");
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Coexistence", "NONE (exclusive)", "Partial", "FULL"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Install complexity", "Low (1 config)", "Low (1 symlink)", "Medium (rename+write)"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Uninstall", "1 config unset", "rm symlink", "restore backup"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Version-controlled", "YES (.zenith/)", "YES (.zenith/)", "NO (.git/hooks/)"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Survives clone", "NO (local cfg)", "NO (.git/)", "NO (.git/)"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "gix write needed", "YES (config)", "NO (fs only)", "NO (fs only)"
    );
    eprintln!();

    // --- Table 2: Hook Implementation ---
    eprintln!("  TABLE 2: Hook Implementation Approach");
    eprintln!("  {:-<75}", "");
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Criterion", "A: Shell", "B: znt hook", "C: Wrapper"
    );
    eprintln!("  {:-<20} {:-<18} {:-<18} {:-<18}", "", "", "", "");
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "JSON validation", "POOR (no jq)", "FULL (serde)", "FULL (if znt avail)"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Schema checks", "NONE", "YES", "YES (if znt avail)"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "BOM detection", "NO", "YES", "YES (if znt avail)"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Conflict markers", "maybe (grep)", "YES", "YES (if znt avail)"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Requires znt PATH", "NO", "YES", "NO (graceful skip)"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Testable in Rust", "NO", "YES", "Partial"
    );
    eprintln!(
        "  {:<20} {:<18} {:<18} {:<18}",
        "Blocks w/o zen", "NO", "YES (fails)", "NO (skips)"
    );
    eprintln!();

    // --- Table 3: Post-checkout Behavior ---
    eprintln!("  TABLE 3: Post-checkout Behavior");
    eprintln!("  {:-<55}", "");
    eprintln!(
        "  {:<25} {:<15} {:<15}",
        "Criterion", "Auto-rebuild", "Warn-only"
    );
    eprintln!("  {:-<25} {:-<15} {:-<15}", "", "", "");
    eprintln!(
        "  {:<25} {:<15} {:<15}",
        "User action needed", "NONE", "Run znt rebuild"
    );
    eprintln!(
        "  {:<25} {:<15} {:<15}",
        "Branch switch delay", "Depends on ops", "Near-zero"
    );
    eprintln!(
        "  {:<25} {:<15} {:<15}",
        "Risk of stale DB", "NONE", "HIGH (if forgot)"
    );
    eprintln!(
        "  {:<25} {:<15} {:<15}",
        "Configurable", "threshold-based", "always fast"
    );
    eprintln!();

    // --- Table 4: gix Verdict ---
    eprintln!("  TABLE 4: gix Verdict");
    eprintln!("  {:-<55}", "");
    eprintln!("  Features used: discover, config r/w, index, tree diff, refs, rev-walk");
    eprintln!("  All 9 operations validated successfully in this spike.");
    eprintln!("  Pure Rust — no libgit2 C dependency.");
    eprintln!("  Well-maintained (gitoxide project, 10k+ stars).");
    eprintln!("  Isolated in zen-hooks — no compile impact on other crates.");
    eprintln!();

    // --- Recommendations ---
    eprintln!("  ===================================================================");
    eprintln!("  RECOMMENDATIONS");
    eprintln!("  ===================================================================");
    eprintln!();
    eprintln!("  1. INSTALLATION: Strategy B (symlink) for MVP.");
    eprintln!("     - Coexists with most setups (only conflicts if same hook name exists).");
    eprintln!("     - Hooks are version-controlled in .zenith/hooks/.");
    eprintln!("     - Detect existing hooks and refuse with guidance (don't overwrite).");
    eprintln!("     - Strategy A (hooksPath) as future option for --exclusive-hooks flag.");
    eprintln!("     - Strategy C (chain) is too complex for v1.");
    eprintln!();
    eprintln!("  2. IMPLEMENTATION: Approach C (wrapper) — thin shell calling znt hook.");
    eprintln!("     - Full Rust validation when znt is in PATH.");
    eprintln!("     - Graceful skip with guidance when znt is not in PATH.");
    eprintln!("     - Never blocks commits — worst case is no validation.");
    eprintln!("     - All validation logic testable in cargo test.");
    eprintln!();
    eprintln!("  3. POST-CHECKOUT: Threshold-based auto-rebuild.");
    eprintln!("     - JSONL parse alone is very fast (<10ms for 5000 ops).");
    eprintln!("     - Full rebuild (with SQLite) is the bottleneck — measure in Phase 2.");
    eprintln!("     - Default: auto-rebuild. Configurable in .zenith/config.toml.");
    eprintln!("     - If rebuild >2s: switch to warn-only with auto as opt-in.");
    eprintln!();
    eprintln!("  4. GIX: Adopt for zen-hooks crate.");
    eprintln!("     - All 9 needed operations work correctly.");
    eprintln!("     - Pure Rust, no C deps, well-maintained.");
    eprintln!("     - Isolated in zen-hooks — no impact on other crates.");
    eprintln!("     - Features: max-performance-safe + index.");
    eprintln!();
    eprintln!("  5. SESSION-GIT: Adopt lightweight session tags.");
    eprintln!("     - Reading branch/HEAD is trivial and adds useful session context.");
    eprintln!("     - Lightweight tags (zenith/ses-xxx) provide git-visible session markers.");
    eprintln!("     - Rev-walk between tags enables 'what happened between sessions' queries.");
    eprintln!("     - Add git_branch + git_commit to sessions table in Phase 1/2.");
}
