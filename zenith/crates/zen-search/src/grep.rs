//! Two-engine grep: `DuckDB` for indexed package source, `grep`+`ignore` for local files.
//!
//! ## Package mode (`grep_package`)
//!
//! Fetches source content from [`SourceFileStore`], applies Rust regex line-by-line,
//! and correlates matches with `api_symbols` via [`ZenLake`] using the
//! `idx_symbols_file_lines` index.
//!
//! ## Local mode (`grep_local`)
//!
//! Uses the `grep` + `ignore` crates (ripgrep's library) for filesystem search,
//! delegating file discovery to [`build_walker`](crate::walk::build_walker).
//! Symbol correlation is NOT available in local mode.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use duckdb::params;
use globset::{Glob, GlobMatcher};
use grep::regex::RegexMatcherBuilder;
use grep::searcher::{BinaryDetection, SearcherBuilder, Sink, SinkContext, SinkMatch};

use zen_lake::{SourceFileStore, ZenLake};

use crate::error::SearchError;
use crate::walk::{WalkMode, build_walker};

// ── Types ──────────────────────────────────────────────────────────

/// A single grep match with optional symbol correlation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepMatch {
    /// File path (relative to package root or local project root).
    pub path: String,
    /// 1-based line number of the match.
    pub line_number: u64,
    /// The matched line text (trimmed trailing whitespace).
    pub text: String,
    /// Context lines before the match.
    pub context_before: Vec<String>,
    /// Context lines after the match.
    pub context_after: Vec<String>,
    /// Enclosing API symbol (package mode only, `None` in local mode).
    pub symbol: Option<SymbolRef>,
}

/// Reference to an API symbol that encloses a grep match.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolRef {
    /// Symbol ID from `api_symbols`.
    pub id: String,
    /// Symbol kind (e.g., "function", "struct", "trait").
    pub kind: String,
    /// Symbol name.
    pub name: String,
    /// Full signature string.
    pub signature: String,
}

/// Result of a grep operation containing matches and statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepResult {
    /// All matches found.
    pub matches: Vec<GrepMatch>,
    /// Aggregate statistics.
    pub stats: GrepStats,
}

/// Statistics about a grep operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrepStats {
    /// Total files searched.
    pub files_searched: u64,
    /// Files with at least one match.
    pub files_matched: u64,
    /// Total number of matches.
    pub matches_found: u64,
    /// Matches that have a symbol correlation.
    pub matches_with_symbol: u64,
    /// Wall-clock time in milliseconds.
    pub elapsed_ms: u64,
}

/// Options controlling grep behavior.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct GrepOptions {
    /// Case-insensitive matching.
    pub case_insensitive: bool,
    /// Auto case-insensitive when pattern is all lowercase.
    pub smart_case: bool,
    /// Treat pattern as literal text, not regex.
    pub fixed_strings: bool,
    /// Whole-word matching.
    pub word_regexp: bool,
    /// Allow matching across line boundaries.
    pub multiline: bool,
    /// Number of context lines before each match.
    pub context_before: u32,
    /// Number of context lines after each match.
    pub context_after: u32,
    /// File glob to include (e.g., `"*.rs"`).
    pub include_glob: Option<String>,
    /// File glob to exclude (e.g., `"tests/"`).
    pub exclude_glob: Option<String>,
    /// Maximum matches per file.
    pub max_count: Option<u32>,
    /// Skip test files and directories.
    pub skip_tests: bool,
    /// Skip symbol correlation (package mode only).
    pub no_symbols: bool,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            case_insensitive: false,
            smart_case: true,
            fixed_strings: false,
            word_regexp: false,
            multiline: false,
            context_before: 2,
            context_after: 2,
            include_glob: None,
            exclude_glob: None,
            max_count: None,
            skip_tests: false,
            no_symbols: false,
        }
    }
}

// ── Internal helpers ───────────────────────────────────────────────

/// Symbol range fetched from `api_symbols`, sorted by `line_start`.
struct SymbolRange {
    id: String,
    kind: String,
    name: String,
    signature: String,
    line_start: i64,
    line_end: i64,
}

/// Find the enclosing symbol for a given line number via linear scan.
///
/// Symbols are sorted by `line_start`. For each match we find the symbol
/// where `line_start <= line <= line_end`.
fn find_enclosing_symbol(symbols: &[SymbolRange], line: u64) -> Option<&SymbolRange> {
    #[allow(clippy::cast_possible_wrap)]
    let line_i64 = line as i64;
    symbols
        .iter()
        .find(|s| line_i64 >= s.line_start && line_i64 <= s.line_end)
}

/// Convert a `duckdb::Error` into `SearchError`.
const fn duck_err(e: duckdb::Error) -> SearchError {
    SearchError::Lake(zen_lake::LakeError::DuckDb(e))
}

// ── Custom Sink for local grep ────────────────────────────────────

/// Intermediate match collected during a file search (before context grouping).
struct RawMatch {
    line_number: u64,
    text: String,
}

/// Intermediate context line collected during a file search.
struct RawContext {
    line_number: u64,
    text: String,
}

/// Custom `Sink` that collects matches and context lines from the grep searcher.
struct MatchCollector {
    matches: Vec<RawMatch>,
    contexts: Vec<RawContext>,
    max_count: Option<u32>,
    count: u32,
}

impl Sink for MatchCollector {
    type Error = io::Error;

    fn matched(
        &mut self,
        _searcher: &grep::searcher::Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, io::Error> {
        if self.max_count.is_some_and(|max| self.count >= max) {
            return Ok(false);
        }
        if let Some(line_num) = mat.line_number() {
            let text = String::from_utf8_lossy(mat.bytes());
            self.matches.push(RawMatch {
                line_number: line_num,
                text: text.trim_end().to_string(),
            });
            self.count += 1;
        }
        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &grep::searcher::Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, io::Error> {
        if let Some(line_num) = ctx.line_number() {
            let text = String::from_utf8_lossy(ctx.bytes());
            self.contexts.push(RawContext {
                line_number: line_num,
                text: text.trim_end().to_string(),
            });
        }
        Ok(true)
    }
}

/// Group raw matches and context lines into [`GrepMatch`] structs.
fn assemble_matches(
    path: &str,
    raw_matches: Vec<RawMatch>,
    raw_contexts: &[RawContext],
    context_before: u32,
    context_after: u32,
) -> Vec<GrepMatch> {
    raw_matches
        .into_iter()
        .map(|m| {
            let before: Vec<String> = raw_contexts
                .iter()
                .filter(|c| {
                    c.line_number < m.line_number
                        && c.line_number >= m.line_number.saturating_sub(u64::from(context_before))
                })
                .map(|c| c.text.clone())
                .collect();

            let after: Vec<String> = raw_contexts
                .iter()
                .filter(|c| {
                    c.line_number > m.line_number
                        && c.line_number <= m.line_number + u64::from(context_after)
                })
                .map(|c| c.text.clone())
                .collect();

            GrepMatch {
                path: path.to_string(),
                line_number: m.line_number,
                text: m.text,
                context_before: before,
                context_after: after,
                symbol: None,
            }
        })
        .collect()
}

// ── GrepEngine ────────────────────────────────────────────────────

/// Stateless two-engine grep. Takes dependencies as parameters.
pub struct GrepEngine;

impl GrepEngine {
    /// Grep indexed package source: `DuckDB` fetch + Rust regex + symbol correlation.
    ///
    /// 1. Query `source_files` from `SourceFileStore` for each specified package
    /// 2. For each file: split content by lines, apply regex, collect matches + context
    /// 3. If `!no_symbols`: batch symbol lookup per matched file via `ZenLake`
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::Grep`] if the pattern fails to compile.
    /// Returns [`SearchError::Lake`] if `DuckDB` queries fail.
    #[allow(clippy::too_many_lines)]
    pub fn grep_package(
        source_store: &SourceFileStore,
        lake: &ZenLake,
        pattern: &str,
        packages: &[(String, String, String)],
        opts: &GrepOptions,
    ) -> Result<GrepResult, SearchError> {
        let start = std::time::Instant::now();

        let matcher = build_matcher(pattern, opts)?;
        let mut searcher = build_searcher(opts);

        let include_matcher = compile_glob(opts.include_glob.as_deref())?;
        let exclude_matcher = compile_glob(opts.exclude_glob.as_deref())?;

        let mut all_matches: Vec<GrepMatch> = Vec::new();
        let mut files_searched: u64 = 0;
        let mut files_matched: u64 = 0;

        let conn = source_store.conn();

        for (ecosystem, package, version) in packages {
            // Fix 1: Query only file_path first, filter, then fetch content
            let mut path_stmt = conn
                .prepare(
                    "SELECT file_path FROM source_files
                     WHERE ecosystem = ? AND package = ? AND version = ?",
                )
                .map_err(duck_err)?;

            let all_paths: Vec<String> = path_stmt
                .query_map(params![ecosystem, package, version], |row| row.get(0))
                .map_err(duck_err)?
                .filter_map(Result::ok)
                .collect();

            // Apply skip_tests, include_glob, and exclude_glob filters
            let filtered_paths: Vec<&str> = all_paths
                .iter()
                .filter(|fp| {
                    if opts.skip_tests && zen_parser::is_test_file(fp) {
                        return false;
                    }
                    if let Some(ref inc) = include_matcher
                        && !inc.is_match(fp)
                    {
                        return false;
                    }
                    if let Some(ref exc) = exclude_matcher
                        && exc.is_match(fp)
                    {
                        return false;
                    }
                    true
                })
                .map(String::as_str)
                .collect();

            // Fetch content only for files that passed filters
            let mut content_stmt = conn
                .prepare(
                    "SELECT file_path, content FROM source_files
                     WHERE ecosystem = ? AND package = ? AND version = ? AND file_path = ?",
                )
                .map_err(duck_err)?;

            // Track matched file paths for batched symbol correlation
            let mut matched_file_paths: Vec<String> = Vec::new();

            for file_path in &filtered_paths {
                let content: String = content_stmt
                    .query_row(params![ecosystem, package, version, file_path], |row| {
                        row.get(1)
                    })
                    .map_err(duck_err)?;

                files_searched += 1;

                let mut collector = MatchCollector {
                    matches: Vec::new(),
                    contexts: Vec::new(),
                    max_count: opts.max_count,
                    count: 0,
                };

                let search_result =
                    searcher.search_slice(&matcher, content.as_bytes(), &mut collector);

                if let Err(e) = search_result {
                    tracing::warn!(file = %file_path, error = %e, "grep search failed for file");
                    continue;
                }

                if collector.matches.is_empty() {
                    continue;
                }

                files_matched += 1;
                matched_file_paths.push((*file_path).to_string());

                let file_matches = assemble_matches(
                    file_path,
                    collector.matches,
                    &collector.contexts,
                    opts.context_before,
                    opts.context_after,
                );

                all_matches.extend(file_matches);
            }

            // Fix 3: Batched symbol correlation — one query for all matched files
            if !opts.no_symbols && !matched_file_paths.is_empty() {
                let symbols_by_file =
                    fetch_batch_symbols(lake, ecosystem, package, version, &matched_file_paths)?;

                for m in &mut all_matches {
                    if let Some(symbols) = symbols_by_file.get(&m.path)
                        && let Some(sym) = find_enclosing_symbol(symbols, m.line_number)
                    {
                        m.symbol = Some(SymbolRef {
                            id: sym.id.clone(),
                            kind: sym.kind.clone(),
                            name: sym.name.clone(),
                            signature: sym.signature.clone(),
                        });
                    }
                }
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        let matches_with_symbol = all_matches.iter().filter(|m| m.symbol.is_some()).count() as u64;

        Ok(GrepResult {
            stats: GrepStats {
                files_searched,
                files_matched,
                #[allow(clippy::cast_possible_truncation)]
                matches_found: all_matches.len() as u64,
                matches_with_symbol,
                #[allow(clippy::cast_possible_truncation)]
                elapsed_ms: start.elapsed().as_millis() as u64,
            },
            matches: all_matches,
        })
    }

    /// Grep local project files using `grep` + `ignore` crates (ripgrep library).
    ///
    /// Uses [`build_walker`] for file discovery and a custom [`Sink`] for matching.
    /// Symbol correlation is NOT available in local mode — all matches have `symbol: None`.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::Grep`] if the pattern fails to compile.
    pub fn grep_local(
        pattern: &str,
        paths: &[PathBuf],
        opts: &GrepOptions,
    ) -> Result<GrepResult, SearchError> {
        let start = std::time::Instant::now();

        let matcher = build_matcher(pattern, opts)?;
        let mut searcher = build_searcher(opts);

        let mut all_matches: Vec<GrepMatch> = Vec::new();
        let mut files_searched: u64 = 0;
        let mut files_matched: u64 = 0;

        for root in paths {
            let walker = build_walker(
                root,
                WalkMode::LocalProject,
                opts.skip_tests,
                opts.include_glob.as_deref(),
                opts.exclude_glob.as_deref(),
            );

            for result in walker {
                let entry = match result {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!(error = %e, "walker error during local grep");
                        continue;
                    }
                };
                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    continue;
                }

                files_searched += 1;

                let rel_path = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or_else(|_| entry.path())
                    .to_string_lossy()
                    .to_string();

                let mut collector = MatchCollector {
                    matches: Vec::new(),
                    contexts: Vec::new(),
                    max_count: opts.max_count,
                    count: 0,
                };

                let search_result = searcher.search_path(
                    &matcher,
                    entry.path(),
                    &mut collector,
                );

                if let Err(e) = search_result {
                    tracing::warn!(path = %rel_path, error = %e, "grep search failed for file");
                    continue;
                }

                if collector.matches.is_empty() {
                    continue;
                }

                files_matched += 1;

                let file_matches = assemble_matches(
                    &rel_path,
                    collector.matches,
                    &collector.contexts,
                    opts.context_before,
                    opts.context_after,
                );

                all_matches.extend(file_matches);
            }
        }

        Ok(GrepResult {
            stats: GrepStats {
                files_searched,
                files_matched,
                #[allow(clippy::cast_possible_truncation)]
                matches_found: all_matches.len() as u64,
                matches_with_symbol: 0,
                #[allow(clippy::cast_possible_truncation)]
                elapsed_ms: start.elapsed().as_millis() as u64,
            },
            matches: all_matches,
        })
    }
}

// ── Shared builder helpers ─────────────────────────────────────────

/// Build a `RegexMatcher` from the pattern and options.
fn build_matcher(
    pattern: &str,
    opts: &GrepOptions,
) -> Result<grep::regex::RegexMatcher, SearchError> {
    let mut builder = RegexMatcherBuilder::new();
    builder
        .case_insensitive(opts.case_insensitive)
        .case_smart(opts.smart_case)
        .fixed_strings(opts.fixed_strings)
        .word(opts.word_regexp)
        .multi_line(opts.multiline);

    builder
        .build(pattern)
        .map_err(|e| SearchError::Grep(format!("invalid pattern: {e}")))
}

/// Build a `Searcher` with the appropriate options.
fn build_searcher(opts: &GrepOptions) -> grep::searcher::Searcher {
    SearcherBuilder::new()
        .line_number(true)
        .before_context(opts.context_before as usize)
        .after_context(opts.context_after as usize)
        .multi_line(opts.multiline)
        .binary_detection(BinaryDetection::quit(0x00))
        .build()
}

/// Compile a glob pattern into a [`GlobMatcher`] for use in package mode filtering.
///
/// Returns `None` if the input is `None`. Uses the `globset` crate for proper
/// glob semantics (supports `**/*.rs`, `src/*.rs`, etc.).
fn compile_glob(pattern: Option<&str>) -> Result<Option<GlobMatcher>, SearchError> {
    pattern
        .map(|p| {
            Glob::new(p)
                .map(|g| g.compile_matcher())
                .map_err(|e| SearchError::Grep(format!("invalid glob pattern: {e}")))
        })
        .transpose()
}

/// Fetch symbols for multiple files in a single `DuckDB` query.
///
/// Returns a map from `file_path` to sorted `Vec<SymbolRange>`.
fn fetch_batch_symbols(
    lake: &ZenLake,
    ecosystem: &str,
    package: &str,
    version: &str,
    file_paths: &[String],
) -> Result<HashMap<String, Vec<SymbolRange>>, SearchError> {
    if file_paths.is_empty() {
        return Ok(HashMap::new());
    }

    let conn = lake.conn();

    // Build parameterised IN clause: WHERE file_path IN (?, ?, ...)
    let placeholders: Vec<&str> = file_paths.iter().map(|_| "?").collect();
    let sql = format!(
        "SELECT file_path, id, kind, name, signature, line_start, line_end
         FROM api_symbols
         WHERE ecosystem = ? AND package = ? AND version = ?
           AND file_path IN ({})
         ORDER BY file_path, line_start",
        placeholders.join(", ")
    );

    let mut stmt = conn.prepare(&sql).map_err(duck_err)?;

    // Build params: [ecosystem, package, version, path1, path2, ...]
    let mut param_values: Vec<Box<dyn duckdb::ToSql>> = Vec::with_capacity(3 + file_paths.len());
    param_values.push(Box::new(ecosystem.to_string()));
    param_values.push(Box::new(package.to_string()));
    param_values.push(Box::new(version.to_string()));
    for fp in file_paths {
        param_values.push(Box::new(fp.clone()));
    }
    let param_refs: Vec<&dyn duckdb::ToSql> = param_values.iter().map(AsRef::as_ref).collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                SymbolRange {
                    id: row.get(1)?,
                    kind: row.get(2)?,
                    name: row.get(3)?,
                    signature: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    line_start: row.get(5)?,
                    line_end: row.get(6)?,
                },
            ))
        })
        .map_err(duck_err)?;

    let mut map: HashMap<String, Vec<SymbolRange>> = HashMap::new();
    for row in rows {
        let (file_path, sym) = row.map_err(duck_err)?;
        map.entry(file_path).or_default().push(sym);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    // ── Test helpers ──────────────────────────────────────────────────

    const SAMPLE_SPAWN: &str = "\
use std::future::Future;

/// Spawns a new asynchronous task.
///
/// This function creates a new task that runs on the runtime.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    // implementation
    crate::runtime::context::spawn(future)
}

/// Spawns a blocking task on a dedicated thread pool.
pub(crate) fn spawn_blocking<F, R>(func: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    // spawn_blocking delegates to the blocking pool
    crate::runtime::blocking::pool::spawn(func)
}
";

    const SAMPLE_RUNTIME: &str = "\
use crate::task;

pub struct Runtime {
    inner: Box<dyn RuntimeInner>,
}

impl Runtime {
    /// Creates a new multi-threaded runtime.
    pub fn new() -> Result<Self, BuildError> {
        Builder::new_multi_thread()
            .enable_all()
            .build()
    }

    /// Spawns a future on this runtime.
    pub fn spawn<F: Future>(&self, future: F) -> JoinHandle<F::Output> {
        self.inner.spawn(future)
    }
}
";

    /// Set up a `SourceFileStore` with sample source files.
    fn setup_source_store() -> SourceFileStore {
        let store = SourceFileStore::open_in_memory().expect("open source store");
        store
            .store_source_files(&[
                zen_lake::SourceFile {
                    ecosystem: "rust".to_string(),
                    package: "tokio".to_string(),
                    version: "1.40.0".to_string(),
                    file_path: "src/task/spawn.rs".to_string(),
                    content: SAMPLE_SPAWN.to_string(),
                    language: Some("rust".to_string()),
                    size_bytes: SAMPLE_SPAWN.len() as i32,
                    line_count: SAMPLE_SPAWN.lines().count() as i32,
                },
                zen_lake::SourceFile {
                    ecosystem: "rust".to_string(),
                    package: "tokio".to_string(),
                    version: "1.40.0".to_string(),
                    file_path: "src/runtime/mod.rs".to_string(),
                    content: SAMPLE_RUNTIME.to_string(),
                    language: Some("rust".to_string()),
                    size_bytes: SAMPLE_RUNTIME.len() as i32,
                    line_count: SAMPLE_RUNTIME.lines().count() as i32,
                },
            ])
            .expect("store source files");
        store
    }

    /// Set up a `ZenLake` with sample symbols for correlation.
    fn setup_lake_with_symbols() -> ZenLake {
        let lake = ZenLake::open_in_memory().expect("open lake");
        let conn = lake.conn();

        // Insert symbols with known line ranges matching SAMPLE_SPAWN
        let symbols = [
            ("sym-1", "function", "spawn", "pub fn spawn<F>(future: F) -> JoinHandle<F::Output>", 6i32, 14i32),
            ("sym-2", "function", "spawn_blocking", "pub(crate) fn spawn_blocking<F, R>(func: F) -> JoinHandle<R>", 17, 24),
        ];

        for (id, kind, name, sig, start, end) in &symbols {
            conn.execute(
                "INSERT INTO api_symbols (id, ecosystem, package, version, file_path, kind, name, signature, line_start, line_end, is_async, is_unsafe, is_error_type, returns_result)
                 VALUES (?, 'rust', 'tokio', '1.40.0', 'src/task/spawn.rs', ?, ?, ?, ?, ?, false, false, false, false)",
                params![id, kind, name, sig, start, end],
            )
            .expect("symbol insert should succeed");
        }

        // Symbols for runtime/mod.rs
        conn.execute(
            "INSERT INTO api_symbols (id, ecosystem, package, version, file_path, kind, name, signature, line_start, line_end, is_async, is_unsafe, is_error_type, returns_result)
             VALUES ('sym-3', 'rust', 'tokio', '1.40.0', 'src/runtime/mod.rs', 'struct', 'Runtime', 'pub struct Runtime', 3, 5, false, false, false, false)",
            [],
        )
        .expect("insert");

        conn.execute(
            "INSERT INTO api_symbols (id, ecosystem, package, version, file_path, kind, name, signature, line_start, line_end, is_async, is_unsafe, is_error_type, returns_result)
             VALUES ('sym-4', 'rust', 'tokio', '1.40.0', 'src/runtime/mod.rs', 'function', 'spawn', 'pub fn spawn<F: Future>(&self, future: F) -> JoinHandle<F::Output>', 16, 18, false, false, false, false)",
            [],
        )
        .expect("insert");

        lake
    }

    fn default_packages() -> Vec<(String, String, String)> {
        vec![("rust".to_string(), "tokio".to_string(), "1.40.0".to_string())]
    }

    // ── Package mode tests ────────────────────────────────────────────

    #[test]
    fn package_basic_pattern_match() {
        let store = setup_source_store();
        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "spawn_blocking",
            &default_packages(),
            &GrepOptions { no_symbols: true, ..Default::default() },
        )
        .unwrap();

        assert!(result.stats.matches_found >= 2, "should find spawn_blocking in multiple lines");
        assert!(result.matches.iter().all(|m| m.text.contains("spawn_blocking")));
    }

    #[test]
    fn package_context_lines() {
        let store = setup_source_store();
        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "spawn_blocking",
            &default_packages(),
            &GrepOptions {
                context_before: 1,
                context_after: 1,
                no_symbols: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(!result.matches.is_empty());
        // At least one match should have context (the fn signature has lines around it)
        let has_context = result.matches.iter().any(|m| {
            !m.context_before.is_empty() || !m.context_after.is_empty()
        });
        assert!(has_context, "at least one match should have context lines");
    }

    #[test]
    fn package_symbol_correlation() {
        let store = setup_source_store();
        let lake = setup_lake_with_symbols();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "spawn",
            &default_packages(),
            &GrepOptions { no_symbols: false, ..Default::default() },
        )
        .unwrap();

        // Some matches should have symbol correlation
        let with_sym: Vec<_> = result.matches.iter().filter(|m| m.symbol.is_some()).collect();
        assert!(!with_sym.is_empty(), "some matches should have symbol refs");

        // Verify spawn_blocking symbol correlation
        let spawn_blocking_match = result
            .matches
            .iter()
            .find(|m| m.symbol.as_ref().is_some_and(|s| s.name == "spawn_blocking"));
        assert!(spawn_blocking_match.is_some(), "should find match inside spawn_blocking");
    }

    #[test]
    fn package_match_outside_symbol_is_none() {
        let store = setup_source_store();
        let lake = setup_lake_with_symbols();

        // "Future" appears on line 1 (use statement) which is outside any symbol range
        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "use std::future::Future",
            &default_packages(),
            &GrepOptions { no_symbols: false, fixed_strings: true, ..Default::default() },
        )
        .unwrap();

        assert!(!result.matches.is_empty());
        // The `use` line is outside any symbol range
        let use_match = result.matches.iter().find(|m| m.text.contains("use std::future"));
        assert!(use_match.is_some_and(|m| m.symbol.is_none()), "use statement should not have a symbol");
    }

    #[test]
    fn package_no_symbols_flag() {
        let store = setup_source_store();
        let lake = setup_lake_with_symbols();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "spawn",
            &default_packages(),
            &GrepOptions { no_symbols: true, ..Default::default() },
        )
        .unwrap();

        assert!(result.matches.iter().all(|m| m.symbol.is_none()), "no_symbols should skip correlation");
        assert_eq!(result.stats.matches_with_symbol, 0);
    }

    #[test]
    fn package_skip_tests() {
        let store = SourceFileStore::open_in_memory().unwrap();
        store
            .store_source_files(&[
                zen_lake::SourceFile {
                    ecosystem: "rust".to_string(),
                    package: "tokio".to_string(),
                    version: "1.40.0".to_string(),
                    file_path: "src/lib.rs".to_string(),
                    content: "pub fn hello() {}\n".to_string(),
                    language: Some("rust".to_string()),
                    size_bytes: 18,
                    line_count: 1,
                },
                zen_lake::SourceFile {
                    ecosystem: "rust".to_string(),
                    package: "tokio".to_string(),
                    version: "1.40.0".to_string(),
                    file_path: "src/lib_test.rs".to_string(),
                    content: "pub fn hello() {}\n".to_string(),
                    language: Some("rust".to_string()),
                    size_bytes: 18,
                    line_count: 1,
                },
            ])
            .unwrap();

        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "hello",
            &default_packages(),
            &GrepOptions { skip_tests: true, no_symbols: true, ..Default::default() },
        )
        .unwrap();

        assert_eq!(result.stats.files_searched, 1, "test file should be skipped");
        assert!(result.matches.iter().all(|m| !m.path.contains("_test.")));
    }

    #[test]
    fn package_case_insensitive() {
        let store = setup_source_store();
        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "SPAWN",
            &default_packages(),
            &GrepOptions {
                case_insensitive: true,
                smart_case: false,
                no_symbols: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(result.stats.matches_found > 0, "case-insensitive should match 'spawn'");
    }

    #[test]
    fn package_fixed_strings() {
        let store = setup_source_store();
        let lake = ZenLake::open_in_memory().unwrap();

        // The pattern has regex metacharacters but should be treated as literal
        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "F::Output",
            &default_packages(),
            &GrepOptions { fixed_strings: true, no_symbols: true, ..Default::default() },
        )
        .unwrap();

        assert!(result.stats.matches_found > 0, "literal 'F::Output' should match");
    }

    #[test]
    fn package_max_count() {
        let store = setup_source_store();
        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "spawn",
            &default_packages(),
            &GrepOptions { max_count: Some(1), no_symbols: true, ..Default::default() },
        )
        .unwrap();

        // Each file should have at most 1 match
        let mut file_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for m in &result.matches {
            *file_counts.entry(m.path.clone()).or_default() += 1;
        }
        for (path, count) in &file_counts {
            assert!(*count <= 1, "file {path} should have at most 1 match, got {count}");
        }
    }

    #[test]
    fn package_include_glob() {
        let store = setup_source_store();
        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "spawn",
            &default_packages(),
            &GrepOptions {
                include_glob: Some("*.rs".to_string()),
                no_symbols: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(result.matches.iter().all(|m| m.path.ends_with(".rs")));
    }

    #[test]
    fn package_exclude_glob() {
        let store = SourceFileStore::open_in_memory().unwrap();
        store
            .store_source_files(&[
                zen_lake::SourceFile {
                    ecosystem: "rust".to_string(),
                    package: "tokio".to_string(),
                    version: "1.40.0".to_string(),
                    file_path: "src/lib.rs".to_string(),
                    content: "pub fn hello() {}\n".to_string(),
                    language: Some("rust".to_string()),
                    size_bytes: 18,
                    line_count: 1,
                },
                zen_lake::SourceFile {
                    ecosystem: "rust".to_string(),
                    package: "tokio".to_string(),
                    version: "1.40.0".to_string(),
                    file_path: "src/runtime/mod.rs".to_string(),
                    content: "pub fn hello() {}\n".to_string(),
                    language: Some("rust".to_string()),
                    size_bytes: 18,
                    line_count: 1,
                },
            ])
            .unwrap();

        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "hello",
            &default_packages(),
            &GrepOptions {
                exclude_glob: Some("**/runtime/**".to_string()),
                no_symbols: true,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(!result.matches.is_empty(), "should have matches from non-excluded files");
        assert!(
            result.matches.iter().all(|m| !m.path.contains("runtime")),
            "runtime files should be excluded"
        );
    }

    #[test]
    fn package_multi_package() {
        let store = SourceFileStore::open_in_memory().unwrap();
        store
            .store_source_files(&[
                zen_lake::SourceFile {
                    ecosystem: "rust".to_string(),
                    package: "tokio".to_string(),
                    version: "1.40.0".to_string(),
                    file_path: "src/lib.rs".to_string(),
                    content: "pub fn spawn() {}\n".to_string(),
                    language: Some("rust".to_string()),
                    size_bytes: 18,
                    line_count: 1,
                },
                zen_lake::SourceFile {
                    ecosystem: "rust".to_string(),
                    package: "serde".to_string(),
                    version: "1.0.210".to_string(),
                    file_path: "src/lib.rs".to_string(),
                    content: "pub trait Serialize {}\n".to_string(),
                    language: Some("rust".to_string()),
                    size_bytes: 22,
                    line_count: 1,
                },
            ])
            .unwrap();

        let lake = ZenLake::open_in_memory().unwrap();

        let packages = vec![
            ("rust".to_string(), "tokio".to_string(), "1.40.0".to_string()),
            ("rust".to_string(), "serde".to_string(), "1.0.210".to_string()),
        ];

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "pub",
            &packages,
            &GrepOptions { no_symbols: true, ..Default::default() },
        )
        .unwrap();

        assert!(result.stats.matches_found >= 2, "should find matches in both packages");
        assert!(result.stats.files_matched >= 2);
    }

    #[test]
    fn package_empty_results() {
        let store = setup_source_store();
        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "definitely_not_in_source_xyz",
            &default_packages(),
            &GrepOptions::default(),
        )
        .unwrap();

        assert_eq!(result.stats.matches_found, 0);
        assert_eq!(result.stats.files_matched, 0);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn package_stats_correct() {
        let store = setup_source_store();
        let lake = setup_lake_with_symbols();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "spawn",
            &default_packages(),
            &GrepOptions::default(),
        )
        .unwrap();

        assert!(result.stats.files_searched >= 2, "should search both files");
        assert!(result.stats.files_matched >= 1, "at least one file should match");
        assert_eq!(result.stats.matches_found, result.matches.len() as u64);
        assert_eq!(
            result.stats.matches_with_symbol,
            result.matches.iter().filter(|m| m.symbol.is_some()).count() as u64
        );
    }

    // ── Local mode tests ──────────────────────────────────────────────

    #[test]
    fn local_basic_match() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "pub fn hello() {}\npub fn world() {}\n").unwrap();

        let result = GrepEngine::grep_local(
            r"pub fn \w+",
            &[tmp.path().to_path_buf()],
            &GrepOptions::default(),
        )
        .unwrap();

        assert_eq!(result.stats.matches_found, 2);
        assert!(result.matches.iter().all(|m| m.symbol.is_none()));
    }

    #[test]
    fn local_gitignore_respected() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::create_dir_all(tmp.path().join("node_modules/pkg")).unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "fn hello() {}\n").unwrap();
        std::fs::write(tmp.path().join("node_modules/pkg/index.js"), "function hello() {}\n").unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "node_modules/\n").unwrap();

        // Init git repo so .gitignore is effective
        let _ = std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(tmp.path())
            .status();

        let result = GrepEngine::grep_local(
            "hello",
            &[tmp.path().to_path_buf()],
            &GrepOptions::default(),
        )
        .unwrap();

        assert!(
            result.matches.iter().all(|m| !m.path.contains("node_modules")),
            "node_modules should be excluded by .gitignore"
        );
    }

    #[test]
    fn local_skip_tests() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::create_dir_all(tmp.path().join("tests")).unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), "fn hello() {}\n").unwrap();
        std::fs::write(tmp.path().join("tests/test.rs"), "fn hello() {}\n").unwrap();

        let result = GrepEngine::grep_local(
            "hello",
            &[tmp.path().to_path_buf()],
            &GrepOptions { skip_tests: true, ..Default::default() },
        )
        .unwrap();

        assert!(
            result.matches.iter().all(|m| !m.path.starts_with("tests")),
            "test directories should be excluded"
        );
    }

    #[test]
    fn local_include_exclude_globs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "fn hello() {}\n").unwrap();
        std::fs::write(tmp.path().join("src/readme.md"), "hello world\n").unwrap();

        let result = GrepEngine::grep_local(
            "hello",
            &[tmp.path().to_path_buf()],
            &GrepOptions {
                include_glob: Some("*.rs".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(result.matches.iter().all(|m| m.path.ends_with(".rs")));
    }

    #[test]
    fn local_symbol_always_none() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.rs"), "pub fn spawn() {}\n").unwrap();

        let result = GrepEngine::grep_local(
            "spawn",
            &[tmp.path().to_path_buf()],
            &GrepOptions::default(),
        )
        .unwrap();

        assert!(!result.matches.is_empty());
        assert!(result.matches.iter().all(|m| m.symbol.is_none()));
        assert_eq!(result.stats.matches_with_symbol, 0);
    }

    #[test]
    fn local_binary_files_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("text.txt"), "hello world\n").unwrap();
        std::fs::write(tmp.path().join("binary.bin"), b"hello\x00world").unwrap();

        let result = GrepEngine::grep_local(
            "hello",
            &[tmp.path().to_path_buf()],
            &GrepOptions::default(),
        )
        .unwrap();

        // Only the text file should match (binary detection quits on NUL)
        assert!(result.matches.iter().all(|m| m.path.contains("text")));
    }

    #[test]
    fn local_stats_correct() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/a.rs"), "fn hello() {}\nfn world() {}\n").unwrap();
        std::fs::write(tmp.path().join("src/b.rs"), "fn other() {}\n").unwrap();

        let result = GrepEngine::grep_local(
            "fn",
            &[tmp.path().to_path_buf()],
            &GrepOptions::default(),
        )
        .unwrap();

        assert!(result.stats.files_searched >= 2);
        assert!(result.stats.files_matched >= 2);
        assert_eq!(result.stats.matches_found, result.matches.len() as u64);
    }

    // ── Edge cases ────────────────────────────────────────────────────

    #[test]
    fn invalid_pattern_returns_error() {
        let store = SourceFileStore::open_in_memory().unwrap();
        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "[invalid(regex",
            &default_packages(),
            &GrepOptions::default(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SearchError::Grep(_)));
    }

    #[test]
    fn word_boundary_matching() {
        let store = SourceFileStore::open_in_memory().unwrap();
        store
            .store_source_files(&[zen_lake::SourceFile {
                ecosystem: "rust".to_string(),
                package: "tokio".to_string(),
                version: "1.40.0".to_string(),
                file_path: "src/lib.rs".to_string(),
                content: "fn spawn() {}\nfn spawn_blocking() {}\nfn respawn() {}\n".to_string(),
                language: Some("rust".to_string()),
                size_bytes: 55,
                line_count: 3,
            }])
            .unwrap();

        let lake = ZenLake::open_in_memory().unwrap();

        let result = GrepEngine::grep_package(
            &store,
            &lake,
            "spawn",
            &default_packages(),
            &GrepOptions {
                word_regexp: true,
                smart_case: false,
                no_symbols: true,
                ..Default::default()
            },
        )
        .unwrap();

        // Only exact word "spawn" should match — not spawn_blocking or respawn
        assert_eq!(result.stats.matches_found, 1);
        assert!(result.matches[0].text.contains("fn spawn()"));
    }
}
