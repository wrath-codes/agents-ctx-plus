use std::sync::OnceLock;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::ui;

pub struct Progress {
    bar: Option<ProgressBar>,
}

static MULTI_PROGRESS: OnceLock<MultiProgress> = OnceLock::new();

fn multi_progress() -> &'static MultiProgress {
    MULTI_PROGRESS.get_or_init(MultiProgress::new)
}

fn terminal_columns() -> Option<usize> {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
}

fn bar_template() -> &'static str {
    match terminal_columns() {
        Some(cols) if cols >= 110 => "{bar:40.cyan/blue} {pos}/{len} {msg}",
        Some(cols) if cols >= 80 => "{wide_bar:.cyan/blue} {pos}/{len} {msg}",
        _ => "{wide_bar:.cyan/blue} {percent}% {msg}",
    }
}

impl Progress {
    #[must_use]
    pub fn spinner(message: &str) -> Self {
        if !ui::prefs().progress {
            return Self { bar: None };
        }

        let bar = multi_progress().add(ProgressBar::new_spinner());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        bar.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        bar.set_message(message.to_string());
        Self { bar: Some(bar) }
    }

    #[must_use]
    pub fn bar(total: u64, message: &str) -> Self {
        if !ui::prefs().progress {
            return Self { bar: None };
        }

        let bar = multi_progress().add(ProgressBar::new(total));
        bar.set_style(
            ProgressStyle::with_template(bar_template())
                .unwrap_or_else(|_| ProgressStyle::default_bar()),
        );
        bar.set_message(message.to_string());
        Self { bar: Some(bar) }
    }

    pub fn set_message(&self, message: &str) {
        if let Some(bar) = &self.bar {
            bar.set_message(message.to_string());
        }
    }

    pub fn inc(&self, delta: u64) {
        if let Some(bar) = &self.bar {
            bar.inc(delta);
        }
    }

    pub fn finish_ok(&self, message: &str) {
        if let Some(bar) = &self.bar {
            bar.finish_with_message(message.to_string());
        }
    }

    pub fn finish_clear(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }

    pub fn finish_err(&self, message: &str) {
        if let Some(bar) = &self.bar {
            bar.abandon_with_message(message.to_string());
        }
    }
}
