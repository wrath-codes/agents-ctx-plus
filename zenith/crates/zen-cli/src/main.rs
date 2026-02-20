#![allow(dead_code)]
#![allow(clippy::nursery)]
#![allow(clippy::pedantic)]
#![allow(clippy::unused_async)]

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

mod bootstrap;
mod cli;
mod commands;
mod context;
mod output;
#[allow(clippy::all)]
mod pipeline;
mod progress;
mod ui;
mod workspace;
mod write_lock;

#[cfg(test)]
mod spike_agentfs;
#[cfg(test)]
mod spike_clap;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("znt error: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    init_tracing(cli.quiet, cli.verbose)?;

    let flags = cli.global_flags();
    ui::init(&flags);

    match &cli.command {
        cli::Commands::Init(args) => return commands::init::handle(args, &flags).await,
        cli::Commands::Hook { action } => return commands::hook::handle(action, &flags).await,
        cli::Commands::Schema(args) => return commands::schema::handle(args, &flags),
        _ => {}
    }

    let config = bootstrap::load_config(&flags).await?;

    if let cli::Commands::Auth { action } = &cli.command {
        return commands::auth::handle(action, &flags, &config).await;
    }

    let project_root = resolve_project_root(flags.project.as_deref())?;
    context::warn_unconfigured(&config);

    let command = cli.command;
    let write_lock = if command_requires_write_lock(&command) {
        Some(write_lock::acquire_for_project(&project_root).await?)
    } else {
        None
    };

    let lake_access_mode = lake_access_mode_for_command(&command);

    let mut ctx = context::AppContext::init(project_root, config, lake_access_mode)
        .await
        .context("failed to initialize zenith application context")?;

    let result = commands::dispatch::dispatch(command, &mut ctx, &flags).await;
    drop(write_lock);
    result
}

fn init_tracing(quiet: bool, verbose: bool) -> anyhow::Result<()> {
    let level = if quiet {
        "error"
    } else if verbose {
        "debug"
    } else {
        "warn"
    };

    let filter = tracing_subscriber::EnvFilter::try_from_env("ZENITH_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))?;

    Ok(())
}

fn resolve_project_root(project_override: Option<&str>) -> anyhow::Result<PathBuf> {
    if let Some(path) = project_override {
        let explicit = PathBuf::from(path);

        if explicit
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ".zenith")
        {
            return explicit
                .parent()
                .map(std::path::Path::to_path_buf)
                .context("invalid --project path: '.zenith' directory has no parent");
        }

        if explicit.join(".zenith").is_dir() {
            return Ok(explicit);
        }

        if explicit.is_dir() {
            return Ok(explicit);
        }

        anyhow::bail!(
            "invalid --project '{}': directory does not exist",
            explicit.display()
        );
    }

    let start = std::env::current_dir().context("failed to read current directory")?;
    context::find_project_root_or_child(&start)
        .context("not a zenith project (no .zenith directory found). Run 'znt init' first.")
}

fn command_requires_write_lock(command: &cli::Commands) -> bool {
    use crate::cli::subcommands::{
        CacheCommands, CompatCommands, FindingCommands, HypothesisCommands, InsightCommands,
        IssueCommands, PrdCommands, ResearchCommands, SessionCommands, StudyCommands, TaskCommands,
        TeamCommands,
    };

    match command {
        cli::Commands::Search(_)
        | cli::Commands::Grep(_)
        | cli::Commands::Audit(_)
        | cli::Commands::WhatsNext => false,
        cli::Commands::Session { action } => !matches!(action, SessionCommands::List { .. }),
        cli::Commands::Cache { action } => matches!(action, CacheCommands::Clean { .. }),
        cli::Commands::Research { action } => !matches!(
            action,
            ResearchCommands::List { .. }
                | ResearchCommands::Get { .. }
                | ResearchCommands::Registry { .. }
        ),
        cli::Commands::Finding { action } => !matches!(
            action,
            FindingCommands::List { .. } | FindingCommands::Get { .. }
        ),
        cli::Commands::Hypothesis { action } => !matches!(
            action,
            HypothesisCommands::List { .. } | HypothesisCommands::Get { .. }
        ),
        cli::Commands::Insight { action } => !matches!(
            action,
            InsightCommands::List { .. } | InsightCommands::Get { .. }
        ),
        cli::Commands::Issue { action } => !matches!(
            action,
            IssueCommands::List { .. } | IssueCommands::Get { .. }
        ),
        cli::Commands::Prd { action } => {
            !matches!(action, PrdCommands::Get { .. } | PrdCommands::List { .. })
        }
        cli::Commands::Task { action } => {
            !matches!(action, TaskCommands::List { .. } | TaskCommands::Get { .. })
        }
        cli::Commands::Compat { action } => !matches!(
            action,
            CompatCommands::List { .. } | CompatCommands::Get { .. }
        ),
        cli::Commands::Study { action } => !matches!(
            action,
            StudyCommands::Get { .. } | StudyCommands::List { .. }
        ),
        cli::Commands::Team { action } => !matches!(action, TeamCommands::List),
        cli::Commands::Log(_)
        | cli::Commands::Link(_)
        | cli::Commands::Unlink(_)
        | cli::Commands::WrapUp(_)
        | cli::Commands::Install(_)
        | cli::Commands::Onboard(_)
        | cli::Commands::Rebuild(_)
        | cli::Commands::Index(_) => true,
        cli::Commands::Init(_)
        | cli::Commands::Hook { .. }
        | cli::Commands::Schema(_)
        | cli::Commands::Auth { .. } => false,
    }
}

fn lake_access_mode_for_command(command: &cli::Commands) -> context::LakeAccessMode {
    if command_requires_write_lock(command) {
        context::LakeAccessMode::ReadWrite
    } else {
        context::LakeAccessMode::ReadOnly
    }
}
