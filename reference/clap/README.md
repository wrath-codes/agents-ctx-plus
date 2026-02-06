# clap

Complete documentation for clap - the fast, flexible, and ergonomic command-line argument parser for Rust.

## Overview

clap is a full-featured, high-performance command-line argument parser for Rust. It provides both a derive macro for declarative CLI definition and a builder API for programmatic CLI construction.

## Key Features

- **Two APIs**: Derive macro (declarative) and Builder API (programmatic)
- **Type Safety**: Compile-time validation of arguments
- **Help Generation**: Automatic `--help` and `--version` flags
- **Shell Completion**: Auto-generate completions for bash, zsh, fish, PowerShell
- **Error Messages**: Beautiful, contextual error messages
- **Performance**: Zero-cost abstractions, minimal runtime overhead
- **Flexibility**: Supports any CLI pattern you can imagine

## Quick Start

### Derive API (Recommended)

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "myapp")]
#[command(about = "A simple CLI app")]
#[command(version = "1.0")]
struct Cli {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,
    
    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
    
    /// Enable verbose mode
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();
    
    for _ in 0..cli.count {
        println!("Hello, {}!", cli.name);
    }
}
```

### Builder API

```rust
use clap::{Arg, Command};

fn main() {
    let cmd = Command::new("myapp")
        .version("1.0")
        .about("A simple CLI app")
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .help("Name of the person to greet")
                .required(true)
        )
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .help("Number of times to greet")
                .default_value("1")
        );
    
    let matches = cmd.get_matches();
    let name = matches.get_one::<String>("name").unwrap();
    let count = matches.get_one::<String>("count").unwrap().parse::<u8>().unwrap();
    
    for _ in 0..count {
        println!("Hello, {}!", name);
    }
}
```

## Installation

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
```

## Documentation Map

```
reference/clap/
├── README.md                 # This file - overview and quick start
├── getting-started/          # Installation and first steps
│   ├── installation.md
│   ├── first-cli.md
│   └── project-setup.md
├── core-concepts/            # Fundamental concepts
│   ├── arguments.md
│   ├── options-and-flags.md
│   ├── subcommands.md
│   └── commands-structure.md
├── features/                 # Feature guides
│   ├── help-and-version.md
│   ├── validation.md
│   ├── shell-completions.md
│   ├── man-pages.md
│   └── error-handling.md
├── derive-macro/             # Derive API documentation
│   ├── overview.md
│   ├── attributes.md
│   ├── field-attributes.md
│   └── advanced-derive.md
├── builder-api/              # Builder API documentation
│   ├── overview.md
│   ├── commands.md
│   ├── arguments.md
│   └── groups.md
├── validation/               # Input validation
│   ├── value-validation.md
│   ├── custom-validators.md
│   └── conflicts.md
├── testing/                  # Testing CLI applications
│   ├── unit-testing.md
│   ├── integration-testing.md
│   └── testing-techniques.md
├── advanced-topics/          # Advanced usage
│   ├── custom-types.md
│   ├── dynamic-cli.md
│   ├── multi-call.md
│   ├── plugins.md
│   └── performance.md
├── examples/                 # Example implementations
│   ├── basic.md
│   ├── intermediate.md
│   └── advanced.md
└── appendix/                 # Reference materials
    ├── cargo-integrations.md
    ├── migration-guide.md
    └── troubleshooting.md
```

## Choosing Between Derive and Builder

### Use Derive When:
- You want declarative, concise code
- Your CLI structure is static
- You prefer compile-time validation
- You want automatic help generation

### Use Builder When:
- You need dynamic CLI construction
- CLI depends on runtime conditions
- You're building CLI frameworks
- You need maximum flexibility

## Comparison with Alternatives

| Feature | clap | structopt | argh | gumdrop |
|---------|------|-----------|------|---------|
| Derive macro | ✅ | ✅ (deprecated) | ✅ | ✅ |
| Builder API | ✅ | ❌ | ❌ | ❌ |
| Subcommands | ✅ | ✅ | ✅ | ⚠️ |
| Validation | ✅ | ✅ | ⚠️ | ⚠️ |
| Shell completions | ✅ | ✅ | ❌ | ❌ |
| Man pages | ✅ | ⚠️ | ❌ | ❌ |
| Error messages | ✅ | ✅ | ⚠️ | ⚠️ |

## Next Steps

1. **[Installation](getting-started/installation.md)** - Install and configure clap
2. **[First CLI](getting-started/first-cli.md)** - Build your first CLI application
3. **[Core Concepts](core-concepts/arguments.md)** - Understand arguments, options, and subcommands
4. **[Derive Macro](derive-macro/overview.md)** - Master the derive API
5. **[Examples](examples/basic.md)** - Learn from complete examples

## Resources

- **GitHub**: https://github.com/clap-rs/clap
- **crates.io**: https://crates.io/crates/clap
- **docs.rs**: https://docs.rs/clap/latest/clap/
- **Examples**: https://github.com/clap-rs/clap/tree/master/examples

## Community

- **Discord**: https://discord.gg/clap-rs
- **Discussions**: GitHub Discussions
- **Issues**: GitHub Issues for bugs and features

---

*clap - Command Line Argument Parser for Rust*