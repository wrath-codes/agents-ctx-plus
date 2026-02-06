# Arguments

## Overview

Arguments are the values passed to your CLI. clap supports several types:

- **Positional Arguments**: Values without flags (e.g., `cmd filename`)
- **Options**: Key-value pairs with flags (e.g., `--name value` or `-n value`)
- **Flags**: Boolean switches (e.g., `--verbose` or `-v`)

## Positional Arguments

### Basic Positional

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// Input file
    input: String,
}
```

```bash
$ myapp file.txt
```

### Multiple Positionals

```rust
#[derive(Parser)]
struct Cli {
    /// Input files
    #[arg(num_args = 1..)]
    files: Vec<String>,
}
```

```bash
$ myapp file1.txt file2.txt file3.txt
```

### Exact Number of Arguments

```rust
#[derive(Parser)]
struct Cli {
    /// Source and destination
    #[arg(num_args = 2)]
    paths: Vec<String>,
}
```

```bash
$ myapp source.txt dest.txt
```

### Value Names

```rust
#[derive(Parser)]
struct Cli {
    /// Input file
    #[arg(value_name = "FILE")]
    input: String,
    
    /// Output file
    #[arg(value_name = "OUTPUT")]
    output: String,
}
```

```bash
$ myapp --help
Usage: myapp [OPTIONS] <FILE> <OUTPUT>
```

## Options

### Short Options

```rust
#[derive(Parser)]
struct Cli {
    /// Port number
    #[arg(short)]
    port: u16,
}
```

```bash
$ myapp -p 8080
```

### Long Options

```rust
#[derive(Parser)]
struct Cli {
    /// Port number
    #[arg(long)]
    port: u16,
}
```

```bash
$ myapp --port 8080
```

### Both Short and Long

```rust
#[derive(Parser)]
struct Cli {
    /// Port number
    #[arg(short, long)]
    port: u16,
}
```

```bash
$ myapp -p 8080
$ myapp --port 8080
```

### Custom Short/Long

```rust
#[derive(Parser)]
struct Cli {
    /// Port number
    #[arg(short = 'P', long = "port-number")]
    port: u16,
}
```

```bash
$ myapp -P 8080
$ myapp --port-number 8080
```

### Optional Options

```rust
#[derive(Parser)]
struct Cli {
    /// Config file path
    #[arg(short, long)]
    config: Option<String>,
}
```

```bash
$ myapp                    # config is None
$ myapp -c config.toml     # config is Some("config.toml")
```

### Default Values

```rust
#[derive(Parser)]
struct Cli {
    /// Port number
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}
```

```rust
#[derive(Parser)]
struct Cli {
    /// Output format
    #[arg(short, long, default_value = "json")]
    format: String,
}
```

### Multiple Values

```rust
#[derive(Parser)]
struct Cli {
    /// Headers to add
    #[arg(short = 'H', long, num_args = 1..)]
    headers: Vec<String>,
}
```

```bash
$ myapp -H "Content-Type: application/json" -H "Authorization: Bearer token"
```

## Flags

### Boolean Flags

```rust
#[derive(Parser)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}
```

```bash
$ myapp          # verbose = false
$ myapp -v       # verbose = true
$ myapp --verbose # verbose = true
```

### Negative Flags

```rust
#[derive(Parser)]
struct Cli {
    /// Use color output
    #[arg(long, default_value_t = true, action = clap::ArgAction::SetFalse)]
    color: bool,
}
```

```bash
$ myapp          # color = true
$ myapp --color  # color = true
$ myapp --no-color # color = false
```

### Count Flags

```rust
#[derive(Parser)]
struct Cli {
    /// Increase verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}
```

```bash
$ myapp          # verbose = 0
$ myapp -v       # verbose = 1
$ myapp -vv      # verbose = 2
$ myapp -vvv     # verbose = 3
```

### SetTrue/SetFalse

```rust
#[derive(Parser)]
struct Cli {
    /// Enable feature
    #[arg(long, action = clap::ArgAction::SetTrue)]
    feature: bool,
}
```

## Argument Types

### Primitive Types

```rust
#[derive(Parser)]
struct Cli {
    /// Integer value
    count: u32,
    
    /// Floating point
    #[arg(short, long)]
    ratio: f64,
    
    /// Boolean flag
    #[arg(short, long)]
    enabled: bool,
    
    /// String value
    #[arg(short, long)]
    name: String,
}
```

### Path Types

```rust
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    /// Input file
    input: PathBuf,
    
    /// Output directory
    #[arg(short, long)]
    output: PathBuf,
}
```

### Enum Types

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
    /// Output format
    #[arg(short, long, value_enum)]
    format: Format,
}
```

```bash
$ myapp --format json
$ myapp --format yaml
$ myapp --help
  --format <FORMAT>  Output format [possible values: json, yaml, toml]
```

### Custom Types

```rust
use std::str::FromStr;

#[derive(Clone)]
struct Port(u16);

impl FromStr for Port {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let port = s.parse::<u16>()
            .map_err(|_| format!("'{}' is not a valid port", s))?;
        
        if port == 0 {
            return Err("Port cannot be 0".to_string());
        }
        
        Ok(Port(port))
    }
}

#[derive(Parser)]
struct Cli {
    /// Server port
    #[arg(short, long)]
    port: Port,
}
```

## Required Arguments

### Required Positional

```rust
#[derive(Parser)]
struct Cli {
    /// Required input file
    input: String,
}
```

### Required Option

```rust
#[derive(Parser)]
struct Cli {
    /// Required API key
    #[arg(short, long, required = true)]
    api_key: String,
}
```

### Conditional Requirements

```rust
#[derive(Parser)]
struct Cli {
    /// Input file (required unless --stdin)
    #[arg(required_unless_present = "stdin")]
    input: Option<String>,
    
    /// Read from stdin
    #[arg(long)]
    stdin: bool,
}
```

## Argument Relationships

### Conflicts

```rust
#[derive(Parser)]
struct Cli {
    /// Input file
    #[arg(short, long, conflicts_with = "stdin")]
    file: Option<String>,
    
    /// Read from stdin
    #[arg(long)]
    stdin: bool,
}
```

### Requires

```rust
#[derive(Parser)]
struct Cli {
    /// Enable encryption
    #[arg(long)]
    encrypt: bool,
    
    /// Encryption key (required if --encrypt)
    #[arg(long, requires = "encrypt")]
    key: Option<String>,
}
```

### Groups

```rust
#[derive(Parser)]
struct Cli {
    #[arg(group = "input")]
    file: Option<String>,
    
    #[arg(group = "input")]
    url: Option<String>,
}
```

## Documentation

### Doc Comments

```rust
#[derive(Parser)]
struct Cli {
    /// Input file to process
    /// 
    /// This file should be in the correct format.
    /// Supported formats: .txt, .md
    input: String,
}
```

### Help Text

```rust
#[derive(Parser)]
struct Cli {
    /// Output file
    #[arg(short, long, help = "Output file path (default: stdout)")]
    output: Option<String>,
}
```

### Long Help

```rust
#[derive(Parser)]
struct Cli {
    /// Optimization level
    #[arg(
        short,
        long,
        help = "Set optimization level",
        long_help = "Set the optimization level for the compiler.

Level 0: No optimization
Level 1: Basic optimization
Level 2: Full optimization
Level 3: Aggressive optimization"
    )]
    opt_level: u8,
}
```

## Next Steps

- **[Options and Flags](options-and-flags.md)** - Advanced option handling
- **[Subcommands](subcommands.md)** - Create command hierarchies
- **[Validation](../validation/value-validation.md)** - Validate input values