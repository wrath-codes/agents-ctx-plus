#![allow(clippy::all)]

//! # Spike 0.9: clap Derive CLI Parsing Validation
//!
//! Validates that `clap` (v4.5, derive feature) compiles and parses the Zenith CLI
//! command structure correctly:
//!
//! - **Derive macros**: `Parser`, `Subcommand`, `Args`, `ValueEnum` all work
//! - **Top-level struct**: Global flags (`--format`, `--limit`, `--quiet`, `--verbose`, `--project`)
//! - **Subcommands**: Unit variants (`WhatsNext`), inline args (`Install`), nested (`Session`, `Finding`)
//! - **ValueEnum**: `OutputFormat` enum restricts `--format` to `json|table|raw`
//! - **Global flags**: Work both before and after the subcommand name
//! - **Positional args**: Single (`Unlink`), multiple (`Link`), mixed with optional flags (`Install`)
//! - **Nested subcommands**: Two-level dispatch (`zen finding create --content "..."`)
//! - **JSON output**: Serde serialization of response structs produces valid JSON
//! - **Error handling**: Missing required args and unknown subcommands are rejected
//!
//! ## Validates
//!
//! CLI framework works with zenith's command structure — blocks Phase 5.
//!
//! ## Design Reference
//!
//! The full CLI design is in `04-cli-api-design.md` (16 domains, ~40+ subcommands).
//! This spike validates a **representative subset** covering every clap pattern used:
//!
//! - Unit variant: `WhatsNext`
//! - Inline args + bool flags: `Init`, `Install`, `WrapUp`
//! - Positional + optional: `Search`, `Link`
//! - Nested subcommands: `Session { SessionCommands }`, `Finding { FindingCommands }`
//!
//! The full enum will be built in Phase 5 (task 5.1) using this exact pattern.
//!
//! ## Clap Notes (v4.5, derive)
//!
//! - `#[arg(global = true)]` makes a flag available before AND after the subcommand
//! - `#[derive(ValueEnum)]` requires `Clone` — produces `[possible values: ...]` in help
//! - `Option<T>` makes a flag optional; bare `T` makes it required
//! - `bool` fields with `#[arg(long)]` become `--flag` (default false, SetTrue action)
//! - `#[command(subcommand)]` on a field delegates to a `#[derive(Subcommand)]` enum
//! - `try_parse_from` returns `Result` — use in tests instead of `parse_from` which exits
//! - Doc comments on fields become help text (`///` → short help, `///\n///` → long help)
//! - Field order determines positional argument order

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

// ── CLI Structs ─────────────────────────────────────────────────────────────

/// Output format for all commands.
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum OutputFormat {
    Json,
    Table,
    Raw,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Raw => write!(f, "raw"),
        }
    }
}

/// Top-level CLI parser for zenith.
#[derive(Parser, Debug)]
#[command(name = "zen", about = "Zenith - developer knowledge toolbox", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format
    #[arg(short, long, global = true, default_value = "json")]
    pub format: OutputFormat,

    /// Max results to return
    #[arg(short, long, global = true)]
    pub limit: Option<u32>,

    /// Quiet mode — suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Verbose mode — include debug info
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Project root path (if not current directory)
    #[arg(short, long, global = true)]
    pub project: Option<String>,
}

/// Top-level command dispatch.
///
/// Representative subset of the full 16-domain command tree.
/// Covers all clap patterns: unit, inline args, positionals, nested subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize zenith for a project
    Init {
        /// Project name (auto-detected from manifest if omitted)
        #[arg(long)]
        name: Option<String>,
        /// Package ecosystem (rust, node, python, elixir)
        #[arg(long)]
        ecosystem: Option<String>,
        /// Skip initial dependency indexing
        #[arg(long)]
        no_index: bool,
    },

    /// Install and index a package
    Install {
        /// Package name (e.g., "tokio", "reqwest")
        package: String,
        /// Package ecosystem
        #[arg(long)]
        ecosystem: Option<String>,
        /// Specific version to index
        #[arg(long)]
        version: Option<String>,
        /// Re-index even if already indexed
        #[arg(long)]
        force: bool,
    },

    /// Search indexed documentation
    Search {
        /// Search query text
        query: String,
        /// Restrict to a specific package
        #[arg(long)]
        package: Option<String>,
        /// Filter by symbol kind (function, struct, trait, etc.)
        #[arg(long)]
        kind: Option<String>,
        /// Search mode (vector, fts, hybrid)
        #[arg(long)]
        mode: Option<String>,
        /// Max tokens to return in context
        #[arg(long)]
        context_budget: Option<u32>,
    },

    /// Session management
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },

    /// Findings — verified facts from research
    Finding {
        #[command(subcommand)]
        action: FindingCommands,
    },

    /// Create entity link
    Link {
        /// Source entity ID (e.g., "hyp-abc123")
        source: String,
        /// Target entity ID (e.g., "fnd-def456")
        target: String,
        /// Relation type (blocks, validates, implements, etc.)
        relation: String,
    },

    /// Remove entity link
    Unlink {
        /// Link ID to remove
        link_id: String,
    },

    /// Project state and recommended next steps
    #[command(name = "whats-next")]
    WhatsNext,

    /// End session, sync to cloud, summarize
    #[command(name = "wrap-up")]
    WrapUp {
        /// Automatically git add + commit at wrap-up
        #[arg(long)]
        auto_commit: bool,
        /// Custom commit message
        #[arg(long)]
        message: Option<String>,
    },
}

/// Session subcommands.
#[derive(Subcommand, Debug)]
pub enum SessionCommands {
    /// Start a new work session
    Start,
    /// End the current session
    End {
        /// Abandon session (discard, don't snapshot)
        #[arg(long)]
        abandon: bool,
    },
    /// List all sessions
    List,
}

/// Finding subcommands — representative of the CRUD + extras pattern
/// used by research, hypothesis, insight, task, compat, and issue domains.
#[derive(Subcommand, Debug)]
pub enum FindingCommands {
    /// Create a new finding
    Create {
        /// Finding content
        #[arg(long)]
        content: String,
        /// Link to research item
        #[arg(long)]
        research: Option<String>,
        /// Source of the finding
        #[arg(long)]
        source: Option<String>,
        /// Confidence level
        #[arg(long)]
        confidence: Option<String>,
        /// Tags to apply (can be repeated)
        #[arg(long)]
        tag: Option<Vec<String>>,
    },
    /// Update an existing finding
    Update {
        /// Finding ID
        id: String,
        /// New content
        #[arg(long)]
        content: Option<String>,
        /// New confidence level
        #[arg(long)]
        confidence: Option<String>,
    },
    /// List findings
    List {
        /// FTS5 search query
        #[arg(long)]
        search: Option<String>,
    },
    /// Get finding details
    Get {
        /// Finding ID
        id: String,
    },
    /// Add a tag to a finding
    Tag {
        /// Finding ID
        id: String,
        /// Tag to add
        tag: String,
    },
    /// Remove a tag from a finding
    Untag {
        /// Finding ID
        id: String,
        /// Tag to remove
        tag: String,
    },
}

// ── Mock Response (for JSON output test) ────────────────────────────────────

/// Example response struct matching the pattern from `04-cli-api-design.md`.
/// Every command returns JSON like this.
#[derive(Serialize, Debug)]
struct FindingResponse {
    id: String,
    content: String,
    confidence: String,
    tags: Vec<String>,
    created_at: String,
}

// ── Tests ───────────────────────────────────────────────────────────────────

// ── 1. Basic subcommand parsing ─────────────────────────────────────────────

/// Parse a unit-variant subcommand with no arguments.
#[test]
fn spike_basic_subcommand_parsing() {
    let cli = Cli::try_parse_from(["zen", "whats-next"]).expect("should parse whats-next");
    assert!(
        matches!(cli.command, Commands::WhatsNext),
        "should match WhatsNext variant"
    );
}

// ── 2. Subcommand with positional and flags ─────────────────────────────────

/// Parse a subcommand with a positional arg and optional flags.
#[test]
fn spike_subcommand_with_positional_and_flags() {
    let cli = Cli::try_parse_from(["zen", "install", "tokio", "--ecosystem", "rust", "--force"])
        .expect("should parse install");

    match &cli.command {
        Commands::Install {
            package,
            ecosystem,
            version,
            force,
        } => {
            assert_eq!(package, "tokio");
            assert_eq!(ecosystem.as_deref(), Some("rust"));
            assert!(version.is_none());
            assert!(force);
        }
        other => panic!("expected Install, got {other:?}"),
    }
}

// ── 3. Nested subcommand ────────────────────────────────────────────────────

/// Parse two-level nested subcommands: `zen session start` and `zen finding create`.
#[test]
fn spike_nested_subcommand() {
    // session start — unit variant
    let cli = Cli::try_parse_from(["zen", "session", "start"]).expect("should parse session start");
    match &cli.command {
        Commands::Session { action } => {
            assert!(
                matches!(action, SessionCommands::Start),
                "should be Start variant"
            );
        }
        other => panic!("expected Session, got {other:?}"),
    }

    // finding create — with args
    let cli = Cli::try_parse_from([
        "zen",
        "finding",
        "create",
        "--content",
        "reqwest supports connection pooling",
        "--tag",
        "verified",
        "--tag",
        "networking",
    ])
    .expect("should parse finding create");

    match &cli.command {
        Commands::Finding { action } => match action {
            FindingCommands::Create { content, tag, .. } => {
                assert_eq!(content, "reqwest supports connection pooling");
                let tags = tag.as_ref().expect("tags should be present");
                assert_eq!(tags, &["verified", "networking"]);
            }
            other => panic!("expected Create, got {other:?}"),
        },
        other => panic!("expected Finding, got {other:?}"),
    }
}

// ── 4. Global flags before subcommand ───────────────────────────────────────

/// Global flags placed BEFORE the subcommand should be captured.
#[test]
fn spike_global_flags_before_subcommand() {
    let cli = Cli::try_parse_from([
        "zen",
        "--format",
        "table",
        "--verbose",
        "--limit",
        "5",
        "whats-next",
    ])
    .expect("should parse global flags before subcommand");

    assert_eq!(cli.format, OutputFormat::Table);
    assert!(cli.verbose);
    assert!(!cli.quiet);
    assert_eq!(cli.limit, Some(5));
    assert!(matches!(cli.command, Commands::WhatsNext));
}

// ── 5. Global flags after subcommand ────────────────────────────────────────

/// Global flags placed AFTER the subcommand should also be captured.
#[test]
fn spike_global_flags_after_subcommand() {
    let cli = Cli::try_parse_from(["zen", "whats-next", "--format", "raw", "--quiet"])
        .expect("should parse global flags after subcommand");

    assert_eq!(cli.format, OutputFormat::Raw);
    assert!(cli.quiet);
    assert!(matches!(cli.command, Commands::WhatsNext));
}

// ── 6. OutputFormat ValueEnum ───────────────────────────────────────────────

/// ValueEnum restricts --format to json, table, raw. Invalid values are rejected.
#[test]
fn spike_output_format_value_enum() {
    // All three valid formats
    for (input, expected) in [
        ("json", OutputFormat::Json),
        ("table", OutputFormat::Table),
        ("raw", OutputFormat::Raw),
    ] {
        let cli = Cli::try_parse_from(["zen", "--format", input, "whats-next"])
            .unwrap_or_else(|e| panic!("should parse format={input}: {e}"));
        assert_eq!(cli.format, expected, "format should be {input}");
    }

    // Invalid format rejected
    let result = Cli::try_parse_from(["zen", "--format", "xml", "whats-next"]);
    assert!(result.is_err(), "invalid format 'xml' should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("invalid value 'xml'"),
        "error should mention invalid value, got: {err}"
    );
}

// ── 7. JSON output serialization ────────────────────────────────────────────

/// Verify that response structs serialize to valid JSON with expected fields.
/// This validates the output pattern, not clap itself.
#[test]
fn spike_json_output_serialization() {
    let response = FindingResponse {
        id: "fnd-abc123".to_string(),
        content: "reqwest supports connection pooling".to_string(),
        confidence: "high".to_string(),
        tags: vec!["verified".to_string(), "networking".to_string()],
        created_at: "2026-02-08T12:00:00Z".to_string(),
    };

    let json = serde_json::to_string_pretty(&response).expect("should serialize to JSON");
    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("output should be valid JSON");

    assert_eq!(parsed["id"], "fnd-abc123");
    assert_eq!(parsed["content"], "reqwest supports connection pooling");
    assert_eq!(parsed["confidence"], "high");
    assert!(parsed["tags"].is_array());
    assert_eq!(parsed["tags"][0], "verified");
    assert_eq!(parsed["tags"][1], "networking");
}

// ── 8. Missing required arg rejected ────────────────────────────────────────

/// `zen install` without the required `package` positional arg should fail.
#[test]
fn spike_missing_required_arg_rejected() {
    let result = Cli::try_parse_from(["zen", "install"]);
    assert!(
        result.is_err(),
        "install without package should be rejected"
    );
}

// ── 9. Unknown subcommand rejected ──────────────────────────────────────────

/// An unknown subcommand should produce an error.
#[test]
fn spike_unknown_subcommand_rejected() {
    let result = Cli::try_parse_from(["zen", "foobar"]);
    assert!(result.is_err(), "unknown subcommand should be rejected");
}

// ── 10. Default values ──────────────────────────────────────────────────────

/// With no optional flags, defaults should apply.
#[test]
fn spike_default_values() {
    let cli = Cli::try_parse_from(["zen", "whats-next"]).expect("should parse with defaults");

    assert_eq!(
        cli.format,
        OutputFormat::Json,
        "default format should be json"
    );
    assert!(!cli.quiet, "quiet should default to false");
    assert!(!cli.verbose, "verbose should default to false");
    assert!(cli.limit.is_none(), "limit should default to None");
    assert!(cli.project.is_none(), "project should default to None");
}

// ── 11. Multiple positional args ────────────────────────────────────────────

/// `zen link` takes 3 positional args: source, target, relation.
#[test]
fn spike_multiple_positional_args() {
    let cli = Cli::try_parse_from(["zen", "link", "hyp-abc123", "fnd-def456", "validates"])
        .expect("should parse link with 3 positionals");

    match &cli.command {
        Commands::Link {
            source,
            target,
            relation,
        } => {
            assert_eq!(source, "hyp-abc123");
            assert_eq!(target, "fnd-def456");
            assert_eq!(relation, "validates");
        }
        other => panic!("expected Link, got {other:?}"),
    }
}

// ── 12. All finding actions parse ───────────────────────────────────────────

/// Validate that all 6 finding subcommands parse correctly.
/// This represents the CRUD + tag/untag pattern reused across 7 domains.
#[test]
fn spike_deeply_nested_finding_actions() {
    // create
    let cli = Cli::try_parse_from(["zen", "finding", "create", "--content", "test finding"])
        .expect("finding create should parse");
    assert!(matches!(
        cli.command,
        Commands::Finding {
            action: FindingCommands::Create { .. }
        }
    ));

    // update
    let cli = Cli::try_parse_from([
        "zen",
        "finding",
        "update",
        "fnd-123",
        "--content",
        "updated",
    ])
    .expect("finding update should parse");
    assert!(matches!(
        cli.command,
        Commands::Finding {
            action: FindingCommands::Update { .. }
        }
    ));

    // list (with optional search)
    let cli = Cli::try_parse_from(["zen", "finding", "list", "--search", "pooling"])
        .expect("finding list should parse");
    match &cli.command {
        Commands::Finding {
            action: FindingCommands::List { search },
        } => {
            assert_eq!(search.as_deref(), Some("pooling"));
        }
        other => panic!("expected Finding List, got {other:?}"),
    }

    // get
    let cli = Cli::try_parse_from(["zen", "finding", "get", "fnd-123"])
        .expect("finding get should parse");
    assert!(matches!(
        cli.command,
        Commands::Finding {
            action: FindingCommands::Get { .. }
        }
    ));

    // tag
    let cli = Cli::try_parse_from(["zen", "finding", "tag", "fnd-123", "verified"])
        .expect("finding tag should parse");
    match &cli.command {
        Commands::Finding {
            action: FindingCommands::Tag { id, tag },
        } => {
            assert_eq!(id, "fnd-123");
            assert_eq!(tag, "verified");
        }
        other => panic!("expected Finding Tag, got {other:?}"),
    }

    // untag
    let cli = Cli::try_parse_from(["zen", "finding", "untag", "fnd-123", "verified"])
        .expect("finding untag should parse");
    match &cli.command {
        Commands::Finding {
            action: FindingCommands::Untag { id, tag },
        } => {
            assert_eq!(id, "fnd-123");
            assert_eq!(tag, "verified");
        }
        other => panic!("expected Finding Untag, got {other:?}"),
    }
}
