use clap::Parser;

pub mod global;
pub mod root_commands;
pub mod subcommands;

pub use global::{GlobalFlags, OutputFormat};
pub use root_commands::Commands;

/// Top-level CLI parser for the `znt` binary.
#[derive(Debug, Parser)]
#[command(name = "znt", version, about = "Zenith - developer knowledge toolbox")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format: json, table, raw
    #[arg(short, long, global = true, default_value = "json")]
    pub format: OutputFormat,

    /// Max results to return
    #[arg(short, long, global = true)]
    pub limit: Option<u32>,

    /// Quiet mode (suppress non-essential output)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Verbose mode (debug logging)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Project root path (defaults to auto-detect via .zenith)
    #[arg(short, long, global = true)]
    pub project: Option<String>,
}

impl Cli {
    /// Extract ergonomic global flags struct for command handlers.
    #[must_use]
    pub fn global_flags(&self) -> GlobalFlags {
        GlobalFlags {
            format: self.format,
            limit: self.limit,
            quiet: self.quiet,
            verbose: self.verbose,
            project: self.project.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, Commands, GlobalFlags, OutputFormat};

    #[test]
    fn clap_command_tree_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn global_flags_parse_before_subcommand() {
        let cli = Cli::try_parse_from([
            "znt",
            "--format",
            "table",
            "--limit",
            "10",
            "--verbose",
            "whats-next",
        ])
        .expect("cli should parse");

        assert_eq!(cli.format, OutputFormat::Table);
        assert_eq!(cli.limit, Some(10));
        assert!(cli.verbose);
        assert!(matches!(cli.command, Commands::WhatsNext));
    }

    #[test]
    fn global_flags_parse_after_subcommand() {
        let cli = Cli::try_parse_from(["znt", "whats-next", "--format", "raw", "--quiet"])
            .expect("cli should parse");

        assert_eq!(cli.format, OutputFormat::Raw);
        assert!(cli.quiet);
        assert!(matches!(cli.command, Commands::WhatsNext));
    }

    #[test]
    fn output_format_rejects_invalid_value() {
        let parsed = Cli::try_parse_from(["znt", "--format", "xml", "whats-next"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn output_format_accepts_all_supported_values() {
        for value in ["json", "table", "raw"] {
            let cli = Cli::try_parse_from(["znt", "--format", value, "whats-next"])
                .expect("cli should parse");
            assert!(matches!(cli.command, Commands::WhatsNext));
        }
    }

    #[test]
    fn global_flags_extraction_copies_values() {
        let cli = Cli::try_parse_from(["znt", "--project", "/tmp/demo", "whats-next"])
            .expect("cli should parse");
        let flags: GlobalFlags = cli.global_flags();
        assert_eq!(flags.project.as_deref(), Some("/tmp/demo"));
    }
}
