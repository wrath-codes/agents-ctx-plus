# Derive Macro Overview

## Overview

The derive macro is the recommended way to define CLI applications with clap. It uses Rust's procedural macros to generate argument parsing code from struct definitions.

## Basic Usage

### Simple CLI

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// Name of the person
    name: String,
    
    /// Number of greetings
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

fn main() {
    let args = Cli::parse();
    for _ in 0..args.count {
        println!("Hello, {}!", args.name);
    }
}
```

### What Derive Generates

The `#[derive(Parser)]` macro generates:

1. **`Parser` trait implementation** - Enables `Cli::parse()`
2. **`CommandFactory` trait** - Enables help generation
3. **`FromArgMatches` trait** - Enables argument extraction
4. **`ValueEnum` implementations** - For enum arguments

## Struct-Level Attributes

### Command Metadata

```rust
#[derive(Parser)]
#[command(name = "myapp")]
#[command(about = "Does something useful")]
#[command(version = "1.0.0")]
#[command(author = "Your Name")]
struct Cli {
    // ...
}
```

### Help Behavior

```rust
#[derive(Parser)]
#[command(
    about = "Short description",
    long_about = "Long description\n\nWith multiple paragraphs",
    help_template = "{before-help}{about-with-newline}{usage-heading}\n{usage}\n\n{all-args}{after-help}",
)]
struct Cli {
    // ...
}
```

### Version Behavior

```rust
#[derive(Parser)]
#[command(version)]                    // Read from Cargo.toml
#[command(version = "1.0.0")]         // Manual version
#[command(long_version = "...")]      // Long version text
struct Cli {
    // ...
}
```

## Field Attributes

### Argument Types

```rust
#[derive(Parser)]
struct Cli {
    /// Positional argument
    input: String,
    
    /// Option with short and long
    #[arg(short, long)]
    output: Option<String>,
    
    /// Flag
    #[arg(short, long)]
    verbose: bool,
    
    /// Multiple values
    #[arg(short, long, num_args = 1..)]
    items: Vec<String>,
    
    /// Default value
    #[arg(short, long, default_value_t = 10)]
    count: u32,
}
```

### Validation

```rust
#[derive(Parser)]
struct Cli {
    /// Required argument
    #[arg(required = true)]
    name: String,
    
    /// Value range
    #[arg(value_parser = clap::value_parser!(u8).range(1..=100))]
    percentage: u8,
    
    /// Conflicts with other arg
    #[arg(conflicts_with = "quiet")]
    verbose: bool,
    
    /// Requires another arg
    #[arg(requires = "config")]
    debug: bool,
}
```

### Documentation

```rust
#[derive(Parser)]
struct Cli {
    /// Short help (shown in -h)
    /// 
    /// Long help (shown in --help)
    /// 
    /// Multiple paragraphs are supported.
    #[arg(short, long)]
    option: String,
    
    /// Custom help text
    #[arg(help = "Custom help", long_help = "Detailed help")]
    custom: String,
    
    /// Hidden from help
    #[arg(hide = true)]
    internal: String,
}
```

## Supported Types

### Primitive Types

```rust
#[derive(Parser)]
struct Cli {
    // Integers
    integer: i32,
    unsigned: u64,
    
    // Floats
    float: f64,
    
    // Boolean
    flag: bool,
    
    // String
    text: String,
    
    // OsString (platform-specific)
    path: std::ffi::OsString,
}
```

### Path Types

```rust
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    input: PathBuf,
    output: PathBuf,
}
```

### Vec<T>

```rust
#[derive(Parser)]
struct Cli {
    // Multiple positional args
    files: Vec<String>,
    
    // Multiple options
    #[arg(short, long)]
    tags: Vec<String>,
}
```

### Option<T>

```rust
#[derive(Parser)]
struct Cli {
    // Optional positional
    name: Option<String>,
    
    // Optional option
    #[arg(short, long)]
    config: Option<String>,
}
```

### Enums

```rust
use clap::ValueEnum;

#[derive(ValueEnum, Clone)]
enum Format {
    Json,
    Yaml,
    Toml,
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, value_enum)]
    format: Format,
}
```

## Custom Types

### FromStr Implementation

```rust
use std::str::FromStr;

#[derive(Clone)]
struct Port(u16);

impl FromStr for Port {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let port = s.parse::<u16>()
            .map_err(|_| format!("Invalid port: {}", s))?;
        
        if port == 0 {
            return Err("Port cannot be 0".to_string());
        }
        
        Ok(Port(port))
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    port: Port,
}
```

### ValueParser

```rust
use clap::builder::PossibleValuesParser;

#[derive(Parser)]
struct Cli {
    #[arg(value_parser = PossibleValuesParser::new(["auto", "never", "always"]))]
    color: String,
}
```

## Subcommands

### Enum Subcommands

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add { name: String },
    Remove { name: String },
}
```

### Struct Subcommands

```rust
#[derive(Args)]
struct AddArgs {
    name: String,
    #[arg(short, long)]
    force: bool,
}

#[derive(Subcommand)]
enum Commands {
    Add(AddArgs),
}
```

## Advanced Patterns

### Flattening

```rust
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    verbose: Verbosity,
    
    #[command(flatten)]
    config: ConfigArgs,
}

#[derive(Args)]
struct Verbosity {
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Args)]
struct ConfigArgs {
    #[arg(short, long)]
    config: Option<String>,
}
```

### External Subcommands

```rust
#[derive(Subcommand)]
enum Commands {
    Known,
    
    #[command(external_subcommand)]
    External(Vec<String>),
}
```

### Subcommand Aliases

```rust
#[derive(Subcommand)]
enum Commands {
    #[command(alias = "ls")]
    List,
    
    #[command(visible_alias = "rm")]
    Remove,
}
```

## Best Practices

### 1. Use Doc Comments

```rust
#[derive(Parser)]
struct Cli {
    /// Input file to process
    /// 
    /// This file should be in the correct format
    input: String,
}
```

### 2. Prefer Strong Types

```rust
// Good: Type-safe
#[derive(ValueEnum, Clone)]
enum Format {
    Json,
    Yaml,
}

#[arg(short, long, value_enum)]
format: Format,

// Bad: String validation at runtime
#[arg(short, long)]
format: String,
```

### 3. Provide Sensible Defaults

```rust
#[arg(short, long, default_value_t = 8080)]
port: u16,
```

### 4. Use Global Arguments

```rust
#[arg(short, long, global = true)]
verbose: bool,
```

## Limitations

- Struct fields must implement `Clone`
- Generic structs are not supported
- Lifetimes in field types are not supported
- Field order determines positional argument order

## Next Steps

- **[Attributes](attributes.md)** - Complete attribute reference
- **[Field Attributes](field-attributes.md)** - Field-level attribute details
- **[Advanced Derive](advanced-derive.md)** - Complex patterns