use clap::{Args, Subcommand};

use crate::cli::subcommands::{
    AuthCommands, CacheCommands, CompatCommands, FindingCommands, HookCommands, HypothesisCommands,
    InsightCommands, IssueCommands, PrdCommands, ResearchCommands, SessionCommands, StudyCommands,
    TaskCommands, TeamCommands,
};

/// Top-level command tree.
#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    /// Initialize zenith for a project.
    Init(InitArgs),
    /// Onboard existing project.
    Onboard(OnboardArgs),
    /// Session management.
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },
    /// Install and index a package.
    Install(InstallArgs),
    /// Search indexed documentation and knowledge.
    Search(SearchArgs),
    /// Regex grep over package cache or local paths.
    Grep(GrepArgs),
    /// Cache management.
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
    /// Research entities.
    Research {
        #[command(subcommand)]
        action: ResearchCommands,
    },
    /// Findings.
    Finding {
        #[command(subcommand)]
        action: FindingCommands,
    },
    /// Hypotheses.
    Hypothesis {
        #[command(subcommand)]
        action: HypothesisCommands,
    },
    /// Insights.
    Insight {
        #[command(subcommand)]
        action: InsightCommands,
    },
    /// Issues.
    Issue {
        #[command(subcommand)]
        action: IssueCommands,
    },
    /// PRD workflow.
    Prd {
        #[command(subcommand)]
        action: PrdCommands,
    },
    /// Tasks.
    Task {
        #[command(subcommand)]
        action: TaskCommands,
    },
    /// Log implementation details for file locations.
    Log(LogArgs),
    /// Compatibility checks.
    Compat {
        #[command(subcommand)]
        action: CompatCommands,
    },
    /// Studies.
    Study {
        #[command(subcommand)]
        action: StudyCommands,
    },
    /// Create an entity link.
    Link(LinkArgs),
    /// Remove an entity link.
    Unlink(UnlinkArgs),
    /// View audit trail.
    Audit(AuditArgs),
    /// Project state and next steps.
    #[command(name = "whats-next")]
    WhatsNext,
    /// End session and perform wrap-up flow.
    #[command(name = "wrap-up")]
    WrapUp(WrapUpArgs),
    /// Rebuild database from JSONL trail files.
    Rebuild(RebuildArgs),
    /// Dump JSON schema for a registered type.
    Schema(SchemaArgs),
    /// Hook handler called by shell wrappers.
    Hook {
        #[command(subcommand)]
        action: HookCommands,
    },
    /// Authentication.
    Auth {
        #[command(subcommand)]
        action: AuthCommands,
    },
    /// Team management.
    Team {
        #[command(subcommand)]
        action: TeamCommands,
    },
    /// Index the current project for private cloud search.
    Index(IndexArgs),
}

/// Arguments for `znt index`.
#[derive(Clone, Debug, Args)]
pub struct IndexArgs {
    /// Path to index (defaults to current project root).
    #[arg(default_value = ".")]
    pub path: String,
    /// Force re-index even if already indexed.
    #[arg(long)]
    pub force: bool,
}

/// Arguments for `znt init`.
#[derive(Clone, Debug, Args)]
pub struct InitArgs {
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub ecosystem: Option<String>,
    #[arg(long)]
    pub no_index: bool,
    #[arg(long)]
    pub skip_hooks: bool,
}

/// Arguments for `znt onboard`.
#[derive(Clone, Debug, Args)]
pub struct OnboardArgs {
    #[arg(long)]
    pub workspace: bool,
    #[arg(long)]
    pub root: Option<String>,
    #[arg(long)]
    pub skip_indexing: bool,
    #[arg(long)]
    pub ecosystem: Option<String>,
    #[arg(long)]
    pub install_hooks: bool,
}

/// Arguments for `znt install`.
#[derive(Clone, Debug, Args)]
pub struct InstallArgs {
    pub package: String,
    #[arg(long)]
    pub ecosystem: Option<String>,
    #[arg(long)]
    pub version: Option<String>,
    #[arg(long)]
    pub include_tests: bool,
    #[arg(long)]
    pub force: bool,
}

/// Arguments for `znt search`.
#[derive(Clone, Debug, Args)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long)]
    pub package: Option<String>,
    #[arg(long)]
    pub ecosystem: Option<String>,
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long)]
    pub mode: Option<String>,
    #[arg(long)]
    pub version: Option<String>,
    #[arg(long)]
    pub context_budget: Option<u32>,
    #[arg(long)]
    pub max_depth: Option<u32>,
    #[arg(long)]
    pub max_chunks: Option<u32>,
    #[arg(long)]
    pub max_bytes_per_chunk: Option<u32>,
    #[arg(long)]
    pub max_total_bytes: Option<u32>,
    #[arg(long)]
    pub show_ref_graph: bool,
}

/// Arguments for `znt grep`.
#[derive(Clone, Debug, Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct GrepArgs {
    pub pattern: String,
    pub paths: Vec<String>,
    #[arg(short = 'P', long = "package")]
    pub packages: Vec<String>,
    #[arg(long)]
    pub ecosystem: Option<String>,
    #[arg(long)]
    pub version: Option<String>,
    #[arg(long)]
    pub all_packages: bool,
    #[arg(short = 'F', long)]
    pub fixed_strings: bool,
    #[arg(short = 'i', long)]
    pub ignore_case: bool,
    #[arg(short = 'S', long, default_value_t = true)]
    pub smart_case: bool,
    #[arg(short = 'w', long)]
    pub word_regexp: bool,
    #[arg(short = 'C', long)]
    pub context: Option<u32>,
    #[arg(long)]
    pub include: Option<String>,
    #[arg(long)]
    pub exclude: Option<String>,
    #[arg(short = 'm', long)]
    pub max_count: Option<u32>,
    #[arg(short = 'c', long)]
    pub count: bool,
    #[arg(long)]
    pub files_with_matches: bool,
    #[arg(long)]
    pub skip_tests: bool,
    #[arg(long)]
    pub no_symbols: bool,
}

/// Arguments for `znt log`.
#[derive(Clone, Debug, Args)]
pub struct LogArgs {
    pub location: String,
    #[arg(long)]
    pub task: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
}

/// Arguments for `znt link`.
#[derive(Clone, Debug, Args)]
pub struct LinkArgs {
    pub source_type: String,
    pub source_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relation: String,
}

/// Arguments for `znt unlink`.
#[derive(Clone, Debug, Args)]
pub struct UnlinkArgs {
    pub link_id: String,
}

/// Arguments for `znt audit`.
#[derive(Clone, Debug, Args)]
pub struct AuditArgs {
    #[arg(long)]
    pub entity_type: Option<String>,
    #[arg(long)]
    pub entity_id: Option<String>,
    #[arg(long)]
    pub action: Option<String>,
    #[arg(long)]
    pub session: Option<String>,
    #[arg(long)]
    pub search: Option<String>,
    #[arg(long)]
    pub files: bool,
    #[arg(long)]
    pub merge_timeline: bool,
}

/// Arguments for `znt wrap-up`.
#[derive(Clone, Debug, Args)]
pub struct WrapUpArgs {
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub auto_commit: bool,
    #[arg(long)]
    pub message: Option<String>,
    #[arg(long)]
    pub require_sync: bool,
}

/// Arguments for `znt rebuild`.
#[derive(Clone, Debug, Args)]
pub struct RebuildArgs {
    #[arg(long)]
    pub trail_dir: Option<String>,
    #[arg(long)]
    pub strict: bool,
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for `znt schema`.
#[derive(Clone, Debug, Args)]
pub struct SchemaArgs {
    pub type_name: String,
}
