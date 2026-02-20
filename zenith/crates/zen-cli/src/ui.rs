use std::io::IsTerminal;
use std::sync::OnceLock;

use crate::cli::{ColorMode, GlobalFlags, OutputFormat, ProgressMode};

#[derive(Clone, Copy, Debug)]
pub struct UiPrefs {
    pub table_color: bool,
    pub progress: bool,
    pub term_width: Option<usize>,
}

static UI_PREFS: OnceLock<UiPrefs> = OnceLock::new();

pub fn init(flags: &GlobalFlags) {
    let is_tty = std::io::stdout().is_terminal();
    let table_color = match flags.color {
        ColorMode::Always => flags.format == OutputFormat::Table,
        ColorMode::Never => false,
        ColorMode::Auto => {
            is_tty
                && flags.format == OutputFormat::Table
                && !flags.quiet
                && std::env::var_os("NO_COLOR").is_none()
        }
    };

    let progress = match flags.progress {
        ProgressMode::On => is_tty && !flags.quiet && flags.format != OutputFormat::Json,
        ProgressMode::Off => false,
        ProgressMode::Auto => is_tty && !flags.quiet && flags.format != OutputFormat::Json,
    };

    let term_width = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 40);

    let _ = UI_PREFS.set(UiPrefs {
        table_color,
        progress,
        term_width,
    });
}

#[must_use]
pub fn prefs() -> UiPrefs {
    *UI_PREFS.get().unwrap_or(&UiPrefs {
        table_color: false,
        progress: false,
        term_width: None,
    })
}
