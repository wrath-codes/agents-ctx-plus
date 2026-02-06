# Basic Examples

## Overview

This section provides basic examples demonstrating common clap usage patterns.

## Example 1: Hello World CLI

### Code

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "hello")]
#[command(about = "A friendly greeting tool")]
#[command(version = "1.0")]
struct Cli {
    /// Name of the person to greet
    name: String,
    
    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
    
    /// Use uppercase
    #[arg(short, long)]
    uppercase: bool,
}

fn main() {
    let cli = Cli::parse();
    
    let mut message = format!("Hello, {}!", cli.name);
    if cli.uppercase {
        message = message.to_uppercase();
    }
    
    for _ in 0..cli.count {
        println!("{}", message);
    }
}
```

### Usage

```bash
# Basic usage
$ hello Alice
Hello, Alice!

# With count
$ hello Alice --count 3
Hello, Alice!
Hello, Alice!
Hello, Alice!

# With uppercase
$ hello Alice -u
HELLO, ALICE!

# Combined options
$ hello Alice -c 2 --uppercase
HELLO, ALICE!
HELLO, ALICE!

# Help
$ hello --help
A friendly greeting tool

Usage: hello [OPTIONS] <NAME>

Arguments:
  <NAME>  Name of the person to greet

Options:
  -c, --count <COUNT>  Number of times to greet [default: 1]
  -u, --uppercase      Use uppercase
  -h, --help           Print help
  -V, --version        Print version
```

## Example 2: File Processor

### Code

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fileproc")]
#[command(about = "Process files")]
struct Cli {
    /// Input file
    input: PathBuf,
    
    /// Output file (default: input + ".out")
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Operation to perform
    #[arg(short, long, value_enum, default_value_t = Operation::Copy)]
    operation: Operation,
    
    /// Overwrite output if exists
    #[arg(short, long)]
    force: bool,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum Operation {
    Copy,
    Move,
    Link,
}

fn main() {
    let cli = Cli::parse();
    
    let output = cli.output.unwrap_or_else(|| {
        let mut path = cli.input.clone();
        path.set_extension("out");
        path
    });
    
    println!("Input: {:?}", cli.input);
    println!("Output: {:?}", output);
    println!("Operation: {:?}", cli.operation);
    println!("Force: {}", cli.force);
}
```

### Usage

```bash
# Copy file
$ fileproc input.txt
Input: "input.txt"
Output: "input.out"
Operation: Copy
Force: false

# Move with custom output
$ fileproc input.txt -o /tmp/output.txt --operation move
Input: "input.txt"
Output: "/tmp/output.txt"
Operation: Move
Force: false

# Force overwrite
$ fileproc input.txt -o existing.txt --force
Input: "input.txt"
Output: "existing.txt"
Operation: Copy
Force: true
```

## Example 3: HTTP Client

### Code

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "httpcli")]
#[command(about = "Simple HTTP client")]
struct Cli {
    /// HTTP method
    #[arg(short, long, value_enum, default_value_t = Method::Get)]
    method: Method,
    
    /// Request URL
    url: String,
    
    /// Request headers
    #[arg(short = 'H', long)]
    headers: Vec<String>,
    
    /// Request body
    #[arg(short, long)]
    body: Option<String>,
    
    /// Timeout in seconds
    #[arg(short, long, default_value_t = 30)]
    timeout: u64,
    
    /// Follow redirects
    #[arg(short = 'L', long)]
    follow_redirects: bool,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

fn main() {
    let cli = Cli::parse();
    
    println!("Method: {:?}", cli.method);
    println!("URL: {}", cli.url);
    println!("Headers: {:?}", cli.headers);
    println!("Body: {:?}", cli.body);
    println!("Timeout: {}s", cli.timeout);
    println!("Follow redirects: {}", cli.follow_redirects);
}
```

### Usage

```bash
# GET request
$ httpcli https://api.example.com/users
Method: Get
URL: https://api.example.com/users
Headers: []
Body: None
Timeout: 30s
Follow redirects: false

# POST with headers and body
$ httpcli -X post https://api.example.com/users \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer token123" \
    -b '{"name": "Alice"}'
Method: Post
URL: https://api.example.com/users
Headers: ["Content-Type: application/json", "Authorization: Bearer token123"]
Body: Some("{\"name\": \"Alice\"}")
Timeout: 30s
Follow redirects: false

# With timeout and redirects
$ httpcli https://example.com --timeout 60 -L
Method: Get
URL: https://example.com
Headers: []
Body: None
Timeout: 60s
Follow redirects: true
```

## Example 4: Configuration CLI

### Code

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "config")]
#[command(about = "Manage configuration")]
struct Cli {
    /// Configuration file
    #[arg(short, long, global = true)]
    file: Option<PathBuf>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// List all configurations
    List,
    /// Remove configuration
    Remove {
        /// Configuration key
        key: String,
    },
}

fn main() {
    let cli = Cli::parse();
    
    println!("Config file: {:?}", cli.file);
    
    match cli.command {
        Commands::Get { key } => {
            println!("Getting: {}", key);
        }
        Commands::Set { key, value } => {
            println!("Setting: {} = {}", key, value);
        }
        Commands::List => {
            println!("Listing all configurations");
        }
        Commands::Remove { key } => {
            println!("Removing: {}", key);
        }
    }
}
```

### Usage

```bash
# Get config
$ config get api.url
Config file: None
Getting: api.url

# Set config with file
$ config -f ./app.toml set database.host localhost
Config file: Some("./app.toml")
Setting: database.host = localhost

# List configs
$ config -f ./app.toml list
Config file: Some("./app.toml")
Listing all configurations

# Remove config
$ config remove old.setting
Config file: None
Removing: old.setting
```

## Example 5: Counter CLI

### Code

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "counter")]
#[command(about = "Count words, lines, or characters")]
struct Cli {
    /// Files to process
    files: Vec<String>,
    
    /// Count lines
    #[arg(short, long)]
    lines: bool,
    
    /// Count words
    #[arg(short, long)]
    words: bool,
    
    /// Count characters
    #[arg(short, long)]
    chars: bool,
}

fn main() {
    let cli = Cli::parse();
    
    // Default to lines, words, and chars if none specified
    let (lines, words, chars) = if !cli.lines && !cli.words && !cli.chars {
        (true, true, true)
    } else {
        (cli.lines, cli.words, cli.chars)
    };
    
    println!("Files: {:?}", cli.files);
    println!("Lines: {}, Words: {}, Chars: {}", lines, words, chars);
}
```

### Usage

```bash
# Default (lines, words, chars)
$ counter file.txt
Files: ["file.txt"]
Lines: true, Words: true, Chars: true

# Count only lines
$ counter -l file.txt
Files: ["file.txt"]
Lines: true, Words: false, Chars: false

# Count lines and words
$ counter -lw file.txt
Files: ["file.txt"]
Lines: true, Words: true, Chars: false

# Multiple files
$ counter file1.txt file2.txt file3.txt
Files: ["file1.txt", "file2.txt", "file3.txt"]
Lines: true, Words: true, Chars: true
```

## Common Patterns

### Pattern 1: Required Arguments

```rust
#[derive(Parser)]
struct Cli {
    /// Required input
    input: String,
    
    /// Optional output (defaults to input + ".out")
    #[arg(short, long)]
    output: Option<String>,
}
```

### Pattern 2: Boolean Flags

```rust
#[derive(Parser)]
struct Cli {
    /// Enable feature
    #[arg(short, long)]
    verbose: bool,
    
    /// Disable feature (negative flag)
    #[arg(long, default_value_t = true, action = clap::ArgAction::SetFalse)]
    color: bool,
}
```

### Pattern 3: Multiple Values

```rust
#[derive(Parser)]
struct Cli {
    /// Multiple files
    #[arg(num_args = 1..)]
    files: Vec<String>,
    
    /// Comma-separated values
    #[arg(short, long, value_delimiter = ',')]
    items: Vec<String>,
}
```

### Pattern 4: Enum Choices

```rust
#[derive(clap::ValueEnum, Clone)]
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

## Next Steps

- **[Intermediate Examples](intermediate.md)** - More complex patterns
- **[Advanced Examples](advanced.md)** - Complex real-world examples