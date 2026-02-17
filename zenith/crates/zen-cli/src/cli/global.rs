use clap::ValueEnum;

/// Shared output mode across all commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Raw,
}

/// Global flags available before or after subcommands.
#[derive(Clone, Debug)]
pub struct GlobalFlags {
    pub format: OutputFormat,
    pub limit: Option<u32>,
    pub quiet: bool,
    pub verbose: bool,
    pub project: Option<String>,
}
