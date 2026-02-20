use clap::ValueEnum;

/// Shared output mode across all commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Raw,
}

/// Progress rendering mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ProgressMode {
    Auto,
    On,
    Off,
}

/// Color rendering mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

/// Global flags available before or after subcommands.
#[derive(Clone, Debug)]
pub struct GlobalFlags {
    pub format: OutputFormat,
    pub limit: Option<u32>,
    pub quiet: bool,
    pub verbose: bool,
    pub project: Option<String>,
    pub progress: ProgressMode,
    pub color: ColorMode,
}
