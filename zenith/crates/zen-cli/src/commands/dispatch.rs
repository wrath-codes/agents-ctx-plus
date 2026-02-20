use crate::cli::GlobalFlags;
use crate::cli::root_commands::Commands;
use crate::commands;
use crate::context::AppContext;

/// Dispatch a parsed command to the corresponding handler module.
pub async fn dispatch(
    command: Commands,
    ctx: &mut AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    match command {
        Commands::Session { action } => commands::session::handle(&action, ctx, flags).await,
        Commands::Research { action } => commands::research::handle(&action, ctx, flags).await,
        Commands::Finding { action } => commands::finding::handle(&action, ctx, flags).await,
        Commands::Hypothesis { action } => commands::hypothesis::handle(&action, ctx, flags).await,
        Commands::Insight { action } => commands::insight::handle(&action, ctx, flags).await,
        Commands::Issue { action } => commands::issue::handle(&action, ctx, flags).await,
        Commands::Prd { action } => commands::prd::handle(&action, ctx, flags).await,
        Commands::Task { action } => commands::task::handle(&action, ctx, flags).await,
        Commands::Log(args) => commands::log::handle(&args, ctx, flags).await,
        Commands::Compat { action } => commands::compat::handle(&action, ctx, flags).await,
        Commands::Study { action } => commands::study::handle(&action, ctx, flags).await,
        Commands::Link(args) => commands::link::handle_link(&args, ctx, flags).await,
        Commands::Unlink(args) => commands::link::handle_unlink(&args, ctx, flags).await,
        Commands::Audit(args) => commands::audit::handle(&args, ctx, flags).await,
        Commands::WhatsNext => commands::whats_next::handle(ctx, flags).await,
        Commands::WrapUp(args) => commands::wrap_up::handle(&args, ctx, flags).await,
        Commands::Search(args) => commands::search::handle(&args, ctx, flags).await,
        Commands::Grep(args) => commands::grep::handle(&args, ctx, flags).await,
        Commands::Cache { action } => commands::cache::handle(&action, ctx, flags).await,
        Commands::Install(args) => commands::install::handle(&args, ctx, flags).await,
        Commands::Onboard(args) => commands::onboard::handle(&args, ctx, flags).await,
        Commands::Rebuild(args) => commands::rebuild::handle(&args, ctx, flags).await,
        Commands::Team { action } => commands::team::handle(&action, ctx, flags).await,
        Commands::Index(args) => commands::index::handle(&args, ctx, flags).await,
        Commands::Init(_) | Commands::Hook { .. } | Commands::Schema(_) | Commands::Auth { .. } => {
            unreachable!("init/hook/schema/auth are pre-dispatched in main")
        }
    }
}
