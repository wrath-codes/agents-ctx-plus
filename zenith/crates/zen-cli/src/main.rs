#![allow(dead_code)]
#![allow(clippy::nursery)]
#![allow(clippy::pedantic)]
#![allow(clippy::unused_async)]

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

mod cli;
mod commands;
mod context;
mod output;
#[allow(clippy::all)]
mod pipeline;

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
    dotenvy::dotenv().ok();

    let cli = cli::Cli::parse();
    init_tracing(cli.quiet, cli.verbose)?;

    let flags = cli.global_flags();

    match &cli.command {
        cli::Commands::Init(args) => return commands::init::handle(args, &flags).await,
        cli::Commands::Hook { action } => return commands::hook::handle(action, &flags).await,
        cli::Commands::Schema(args) => return commands::schema::handle(args, &flags),
        _ => {}
    }

    let project_root = resolve_project_root(flags.project.as_deref())?;
    let config = zen_config::ZenConfig::load().map_err(anyhow::Error::from)?;
    context::warn_unconfigured(&config);

    let mut ctx = context::AppContext::init(project_root, config)
        .await
        .context("failed to initialize zenith application context")?;

    commands::dispatch::dispatch(cli.command, &mut ctx, &flags).await
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
    let start = match project_override {
        Some(path) => PathBuf::from(path),
        None => std::env::current_dir().context("failed to read current directory")?,
    };

    context::find_project_root(&start)
        .context("not a zenith project (no .zenith directory found). Run 'znt init' first.")
}
