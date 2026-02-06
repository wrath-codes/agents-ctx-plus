# Your First CLI

## Overview

This guide walks you through building your first CLI application with clap. By the end, you'll have a fully functional command-line tool.

## Creating the Project

```bash
# Create new project
cargo new hello-cli
cd hello-cli

# Add clap
cargo add clap --features derive
```

## Basic CLI

### 1. Positional Arguments

```rust
// src/main.rs
use clap::Parser;

#[derive(Parser)]
#[command(name = "hello")]
#[command(about = "Says hello")]
struct Cli {
    /// Name of the person to greet
    name: String,
}

fn main() {
    let cli = Cli::parse();
    println!("Hello, {}!", cli.name);
}
```

**Usage:**
```bash
$ cargo run -- Alice
Hello, Alice!

$ cargo run --
error: The following required argument was not provided: name
```

### 2. Adding Options

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "hello")]
#[command(about = "Says hello")]
struct Cli {
    /// Name of the person to greet
    name: String,
    
    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

fn main() {
    let cli = Cli::parse();
    
    for _ in 0..cli.count {
        println!("Hello, {}!", cli.name);
    }
}
```

**Usage:**
```bash
$ cargo run -- Alice
Hello, Alice!

$ cargo run -- Alice --count 3
Hello, Alice!
Hello, Alice!
Hello, Alice!

$ cargo run -- Alice -c 3
Hello, Alice!
Hello, Alice!
Hello, Alice!
```

### 3. Adding Flags

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "hello")]
#[command(about = "Says hello")]
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
    
    let message = format!("Hello, {}!", cli.name);
    let message = if cli.uppercase {
        message.to_uppercase()
    } else {
        message
    };
    
    for _ in 0..cli.count {
        println!("{}", message);
    }
}
```

**Usage:**
```bash
$ cargo run -- Alice
Hello, Alice!

$ cargo run -- Alice -u
HELLO, ALICE!

$ cargo run -- Alice --uppercase
HELLO, ALICE!
```

## Adding Help

### Automatic Help

clap automatically generates help:

```bash
$ cargo run -- --help
Says hello

Usage: hello [OPTIONS] <NAME>

Arguments:
  <NAME>  Name of the person to greet

Options:
  -c, --count <COUNT>  Number of times to greet [default: 1]
  -u, --uppercase      Use uppercase
  -h, --help           Print help
  -V, --version        Print version
```

### Custom Help Text

```rust
#[derive(Parser)]
#[command(
    name = "hello",
    about = "A friendly greeting tool",
    long_about = "A friendly greeting tool that says hello to people.

This tool demonstrates basic clap functionality including:
- Positional arguments
- Options with short and long flags
- Boolean flags
- Default values",
)]
struct Cli {
    /// Name of the person to greet
    /// 
    /// This should be the person's first name.
    name: String,
    
    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}
```

## Adding Version

### From Cargo.toml

```rust
#[derive(Parser)]
#[command(version)]  // Uses version from Cargo.toml
struct Cli {
    name: String,
}
```

### Manual Version

```rust
#[derive(Parser)]
#[command(version = "1.0.0")]
struct Cli {
    name: String,
}
```

**Usage:**
```bash
$ cargo run -- --version
hello 0.1.0
```

## Optional Arguments

### Optional Positional

```rust
#[derive(Parser)]
struct Cli {
    /// Name of the person (optional)
    name: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.name {
        Some(name) => println!("Hello, {}!", name),
        None => println!("Hello, World!"),
    }
}
```

### Optional with Default

```rust
#[derive(Parser)]
struct Cli {
    /// Name of the person (defaults to "World")
    #[arg(default_value = "World")]
    name: String,
}
```

## Multiple Values

```rust
#[derive(Parser)]
struct Cli {
    /// Names of people to greet
    #[arg(num_args = 1..)]
    names: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    
    for name in cli.names {
        println!("Hello, {}!", name);
    }
}
```

**Usage:**
```bash
$ cargo run -- Alice Bob Charlie
Hello, Alice!
Hello, Bob!
Hello, Charlie!
```

## Complete Example

```rust
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "hello",
    version,
    about = "A friendly greeting tool",
)]
struct Cli {
    /// Names of people to greet
    #[arg(required = true, num_args = 1..)]
    names: Vec<String>,
    
    /// Number of times to greet each person
    #[arg(short, long, default_value_t = 1, value_name = "N")]
    count: u8,
    
    /// Use uppercase
    #[arg(short, long)]
    uppercase: bool,
    
    /// Don't print newlines
    #[arg(short, long)]
    no_newline: bool,
}

fn main() {
    let cli = Cli::parse();
    
    let ending = if cli.no_newline { "" } else { "\n" };
    
    for _ in 0..cli.count {
        for name in &cli.names {
            let mut message = format!("Hello, {}!", name);
            
            if cli.uppercase {
                message = message.to_uppercase();
            }
            
            print!("{}{}", message, ending);
        }
    }
}
```

**Usage:**
```bash
$ cargo run -- Alice Bob -c 2 -u
HELLO, ALICE!
HELLO, BOB!
HELLO, ALICE!
HELLO, BOB!
```

## Next Steps

- **[Project Setup](project-setup.md)** - Organize larger projects
- **[Core Concepts](../core-concepts/arguments.md)** - Learn about arguments in depth
- **[Examples](../examples/basic.md)** - See more examples