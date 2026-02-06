# Project Setup

## Project Structure

For CLI applications, a well-organized project structure improves maintainability:

```
my-cli/
├── Cargo.toml
├── src/
│   ├── main.rs          # Entry point
│   ├── cli.rs           # CLI definition (clap)
│   ├── commands/        # Command implementations
│   │   ├── mod.rs
│   │   ├── add.rs
│   │   ├── remove.rs
│   │   └── list.rs
│   ├── lib.rs           # Library exports (optional)
│   └── utils.rs         # Helper functions
├── tests/               # Integration tests
│   └── integration_tests.rs
├── examples/            # Usage examples
│   └── basic.rs
├── completions/         # Shell completion scripts
│   ├── bash.sh
│   ├── zsh.zsh
│   └── fish.fish
└── man/                 # Man pages
    └── my-cli.1
```

## Cargo.toml Structure

### Basic CLI

```toml
[package]
name = "my-cli"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <you@example.com>"]
description = "A CLI tool that does something"
license = "MIT OR Apache-2.0"
repository = "https://github.com/username/my-cli"
keywords = ["cli", "tool"]
categories = ["command-line-utilities"]
rust-version = "1.70"  # MSRV

[dependencies]
clap = { version = "4.5", features = ["derive", "cargo", "env"] }

[dev-dependencies]
assert_cmd = "2.0"      # For testing
predicates = "3.0"      # For assertions

[[bin]]
name = "my-cli"
path = "src/main.rs"
```

### Multi-Command CLI

```toml
[package]
name = "my-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive", "cargo", "env"] }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

[[bin]]
name = "my-cli"
path = "src/main.rs"

[features]
default = ["full"]
full = []
minimal = []

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

## Module Organization

### cli.rs - CLI Definition

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "my-cli")]
#[command(about = "A CLI tool")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new item
    Add(crate::commands::add::Args),
    /// Remove an item
    Remove(crate::commands::remove::Args),
    /// List all items
    List(crate::commands::list::Args),
}
```

### commands/mod.rs

```rust
pub mod add;
pub mod remove;
pub mod list;

use clap::Parser;

/// Trait for command execution
pub trait Command {
    type Args: Parser;
    
    fn execute(args: Self::Args) -> anyhow::Result<()>;
}
```

### commands/add.rs

```rust
use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// Item name
    pub name: String,
    
    /// Item description
    #[arg(short, long)]
    pub description: Option<String>,
}

pub fn execute(args: Args) -> anyhow::Result<()> {
    println!("Adding: {}", args.name);
    
    if let Some(desc) = args.description {
        println!("Description: {}", desc);
    }
    
    Ok(())
}
```

### main.rs

```rust
mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Set up logging based on verbose flag
    if cli.verbose {
        env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }
    
    match cli.command {
        Commands::Add(args) => commands::add::execute(args),
        Commands::Remove(args) => commands::remove::execute(args),
        Commands::List(args) => commands::list::execute(args),
    }
}
```

## Error Handling

### Using anyhow

```toml
[dependencies]
anyhow = "1.0"
```

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Add(args) => add(args).context("Failed to add item")?,
        Commands::Remove(args) => remove(args).context("Failed to remove item")?,
    }
    
    Ok(())
}
```

### Using thiserror

```toml
[dependencies]
thiserror = "1.0"
```

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Item not found: {0}")]
    NotFound(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    // ...
    Ok(())
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_command() {
        let args = commands::add::Args {
            name: "test".to_string(),
            description: None,
        };
        
        assert!(commands::add::execute(args).is_ok());
    }
}
```

### Integration Tests

```rust
// tests/integration_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_basic() {
    let mut cmd = Command::cargo_bin("my-cli").unwrap();
    
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A CLI tool"));
}

#[test]
fn test_add_command() {
    let mut cmd = Command::cargo_bin("my-cli").unwrap();
    
    cmd.args(["add", "test-item"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Adding: test-item"));
}

#[test]
fn test_missing_argument() {
    let mut cmd = Command::cargo_bin("my-cli").unwrap();
    
    cmd.arg("add");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}
```

## Documentation

### Cargo.toml Metadata

```toml
[package.metadata.deb]
section = "utils"
priority = "optional"
assets = [
    ["target/release/my-cli", "usr/bin/", "755"],
    ["README.md", "usr/share/doc/my-cli/", "644"],
]

[package.metadata.wix]
upgrade-guid = "SOME-GUID-HERE"
path-guid = "ANOTHER-GUID"
```

### README.md Template

```markdown
# My CLI

[![Crates.io](https://img.shields.io/crates/v/my-cli)](https://crates.io/crates/my-cli)
[![Documentation](https://docs.rs/my-cli/badge.svg)](https://docs.rs/my-cli)

A CLI tool that does something useful.

## Installation

```bash
cargo install my-cli
```

## Usage

```bash
my-cli --help
```

## Examples

### Add an item

```bash
my-cli add "New Item" --description "A description"
```

## License

MIT OR Apache-2.0
```

## Build Scripts

### Generating Shell Completions

```rust
// build.rs
use std::io;

fn main() -> io::Result<()> {
    let outdir = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "OUT_DIR not set")
        })?
    );
    
    let cmd = clap::Command::new("my-cli")
        .version("1.0")
        .about("A CLI tool");
    
    // Generate completions
    let shells = [
        clap_complete::Shell::Bash,
        clap_complete::Shell::Zsh,
        clap_complete::Shell::Fish,
        clap_complete::Shell::PowerShell,
    ];
    
    for shell in shells {
        clap_complete::generate_to(
            shell,
            &mut cmd.clone(),
            "my-cli",
            &outdir,
        )?;
    }
    
    Ok(())
}
```

```toml
[build-dependencies]
clap_complete = "4.5"
```

## Distribution

### Cargo Install

```bash
cargo install --path .
```

### Binary Releases

```bash
# Build release binary
cargo build --release

# Strip symbols (optional)
strip target/release/my-cli
```

### Cross-Compilation

```bash
# Install cross
cargo install cross

# Build for different targets
cross build --release --target x86_64-unknown-linux-musl
cross build --release --target x86_64-pc-windows-gnu
cross build --release --target x86_64-apple-darwin
```

## Next Steps

- **[Core Concepts](../core-concepts/arguments.md)** - Learn about arguments in depth
- **[Derive Macro](../derive-macro/overview.md)** - Master the derive API
- **[Testing](../testing/integration-testing.md)** - Write comprehensive tests