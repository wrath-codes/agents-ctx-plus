# Options and Flags

## Overview

Options and flags are the most common way users interact with CLI applications. This guide covers advanced patterns for handling them.

## Options

### Option Types

#### Optional Value

```rust
#[derive(Parser)]
struct Cli {
    /// Limit (optional)
    #[arg(short, long)]
    limit: Option<u32>,
}

// Usage:
// myapp           -> limit = None
// myapp -l 10     -> limit = Some(10)
// myapp --limit 10 -> limit = Some(10)
```

#### Required Value

```rust
#[derive(Parser)]
struct Cli {
    /// API key (required)
    #[arg(short, long, required = true)]
    api_key: String,
}
```

#### With Default

```rust
#[derive(Parser)]
struct Cli {
    /// Number of retries
    #[arg(short, long, default_value_t = 3)]
    retries: u32,
    
    /// Output format
    #[arg(short, long, default_value = "text")]
    format: String,
}
```

### Multiple Options

#### Vec<T>

```rust
#[derive(Parser)]
struct Cli {
    /// Define variables
    #[arg(short = 'D', long, num_args = 1..)]
    defines: Vec<String>,
}

// Usage:
// myapp -D DEBUG=1 -D VERSION=2.0
```

#### Delimited Values

```rust
#[derive(Parser)]
struct Cli {
    /// Comma-separated values
    #[arg(short, long, value_delimiter = ',')]
    items: Vec<String>,
}

// Usage:
// myapp -i "a,b,c"
// items = vec!["a", "b", "c"]
```

### Value Parsing

#### Range Validation

```rust
#[derive(Parser)]
struct Cli {
    /// Port number (1-65535)
    #[arg(short, long, value_parser = clap::value_parser!(u16).range(1..))]
    port: u16,
    
    /// Percentage (0-100)
    #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
    percentage: u8,
}
```

#### Custom Parser

```rust
fn parse_duration(s: &str) -> Result<std::time::Duration, String> {
    let num: u64 = s.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .map_err(|_| format!("Invalid duration: {}", s))?;
    
    let unit = s.chars()
        .skip_while(|c| c.is_ascii_digit())
        .collect::<String>();
    
    match unit.as_str() {
        "s" => Ok(std::time::Duration::from_secs(num)),
        "m" => Ok(std::time::Duration::from_secs(num * 60)),
        "h" => Ok(std::time::Duration::from_secs(num * 3600)),
        _ => Err(format!("Invalid unit: {}", unit)),
    }
}

#[derive(Parser)]
struct Cli {
    /// Timeout duration (e.g., 30s, 5m, 1h)
    #[arg(short, long, value_parser = parse_duration)]
    timeout: std::time::Duration,
}

// Usage:
// myapp --timeout 30s
// myapp --timeout 5m
// myapp --timeout 1h
```

### Environment Variables

#### Basic Usage

```rust
#[derive(Parser)]
struct Cli {
    /// API key
    #[arg(short, long, env = "API_KEY")]
    api_key: String,
}

// Can be set via:
// API_KEY=secret myapp
// or
// myapp --api-key secret
```

#### With Fallback

```rust
#[derive(Parser)]
struct Cli {
    /// Configuration file
    #[arg(
        short,
        long,
        env = "CONFIG_FILE",
        default_value = "/etc/myapp/config.toml"
    )]
    config: String,
}
```

#### Hide from Help

```rust
#[derive(Parser)]
struct Cli {
    /// Secret key (don't show in help)
    #[arg(long, env = "SECRET_KEY", hide_env_values = true)]
    secret: String,
}
```

## Flags

### Boolean Flags

#### Simple Flag

```rust
#[derive(Parser)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

// Usage:
// myapp           -> verbose = false
// myapp -v        -> verbose = true
// myapp --verbose -> verbose = true
```

#### Negative Flag

```rust
#[derive(Parser)]
struct Cli {
    /// Use color output [default: true]
    #[arg(long, default_value_t = true, action = clap::ArgAction::SetFalse)]
    color: bool,
}

// Usage:
// myapp             -> color = true
// myapp --color     -> color = true
// myapp --no-color  -> color = false
```

#### Counting Flag

```rust
#[derive(Parser)]
struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

// Usage:
// myapp      -> verbose = 0
// myapp -v   -> verbose = 1
// myapp -vv  -> verbose = 2
// myapp -vvv -> verbose = 3

fn main() {
    let cli = Cli::parse();
    
    match cli.verbose {
        0 => println!("Error level"),
        1 => println!("Warning level"),
        2 => println!("Info level"),
        3 => println!("Debug level"),
        _ => println!("Trace level"),
    }
}
```

### Action Types

#### SetTrue

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, action = clap::ArgAction::SetTrue)]
    feature: bool,
}
```

#### SetFalse

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, default_value_t = true, action = clap::ArgAction::SetFalse)]
    enable: bool,
}
```

#### Append

```rust
#[derive(Parser)]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Append)]
    item: Vec<String>,
}

// Usage:
// myapp -i a -i b -i c
// items = vec!["a", "b", "c"]
```

## Combining Options

### Short Option Groups

```rust
#[derive(Parser)]
struct Cli {
    #[arg(short)]
    verbose: bool,
    
    #[arg(short)]
    quiet: bool,
    
    #[arg(short)]
    force: bool,
}

// Usage:
// myapp -vqf (equivalent to -v -q -f)
```

### Option Relationships

#### Conflicts

```rust
#[derive(Parser)]
struct Cli {
    /// Read from file
    #[arg(short, long, conflicts_with = "stdin")]
    file: Option<String>,
    
    /// Read from stdin
    #[arg(long)]
    stdin: bool,
}

// Error:
// myapp -f file.txt --stdin
// error: The argument '--stdin' cannot be used with '--file <FILE>'
```

#### Requires

```rust
#[derive(Parser)]
struct Cli {
    /// Enable SSL
    #[arg(long)]
    ssl: bool,
    
    /// SSL certificate (requires --ssl)
    #[arg(long, requires = "ssl")]
    cert: Option<String>,
}
```

#### Required Together

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, required_if_eq_all = [("auth", "basic"), ("user", None)])]
    password: Option<String>,
    
    #[arg(long)]
    user: Option<String>,
    
    #[arg(long)]
    auth: Option<String>,
}
```

## Advanced Patterns

### Conditional Defaults

```rust
#[derive(Parser)]
struct Cli {
    /// Output file (default: input + ".out")
    #[arg(short, long)]
    output: Option<String>,
    
    input: String,
}

fn main() {
    let mut cli = Cli::parse();
    
    if cli.output.is_none() {
        cli.output = Some(format!("{}.out", cli.input));
    }
}
```

### Override Help

```rust
#[derive(Parser)]
struct Cli {
    /// Config file
    #[arg(
        short,
        long,
        help = "Path to configuration file",
        long_help = "Path to the configuration file.

If not provided, the following locations are searched:
1. ./config.toml
2. ~/.config/myapp/config.toml
3. /etc/myapp/config.toml"
    )]
    config: Option<String>,
}
```

### Hidden Options

```rust
#[derive(Parser)]
struct Cli {
    /// Debug mode (hidden from help)
    #[arg(long, hide = true)]
    debug: bool,
    
    /// Internal use only
    #[arg(long, hide = true)]
    internal_id: Option<String>,
}
```

## Best Practices

### 1. Use Common Conventions

```rust
#[derive(Parser)]
struct Cli {
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Quiet output (overrides verbose)
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,
    
    /// Configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,
    
    /// Output file
    #[arg(short, long, value_name = "FILE")]
    output: Option<String>,
}
```

### 2. Provide Sensible Defaults

```rust
#[derive(Parser)]
struct Cli {
    /// Number of retries
    #[arg(short, long, default_value_t = 3)]
    retries: u32,
    
    /// Timeout in seconds
    #[arg(short, long, default_value_t = 30)]
    timeout: u64,
}
```

### 3. Use Env Vars for Secrets

```rust
#[derive(Parser)]
struct Cli {
    /// API key
    #[arg(long, env = "API_KEY", hide_env_values = true)]
    api_key: String,
}
```

### 4. Group Related Options

```rust
#[derive(Parser)]
struct Cli {
    /// Input options
    #[arg(short, long, group = "input")]
    file: Option<String>,
    
    #[arg(long, group = "input")]
    url: Option<String>,
    
    /// Output options
    #[arg(short, long)]
    output: Option<String>,
    
    #[arg(long)]
    stdout: bool,
}
```

## Next Steps

- **[Subcommands](subcommands.md)** - Create command hierarchies
- **[Validation](../validation/value-validation.md)** - Validate input values
- **[Derive Macro](../derive-macro/attributes.md)** - Advanced derive patterns