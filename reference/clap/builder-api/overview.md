# Builder API Overview

## Overview

The Builder API provides a programmatic way to define CLI applications. While more verbose than the derive macro, it offers maximum flexibility and dynamic CLI construction.

## Basic Usage

### Simple CLI

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
                .help("Name of the person")
                .required(true)
        )
        .arg(
            Arg::new("count")
                .short('c')
                .long("count")
                .help("Number of times")
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

## Command Construction

### Building Commands

```rust
let cmd = Command::new("myapp")
    // Metadata
    .version("1.0.0")
    .author("Author Name")
    .about("Does something")
    .long_about("Detailed description")
    
    // Settings
    .color(clap::ColorChoice::Auto)
    .term_width(80)
    
    // Arguments
    .arg(Arg::new("input")
        .help("Input file")
        .required(true)
    );
```

### Chaining Pattern

```rust
let cmd = Command::new("myapp")
    .version("1.0")
    .about("A CLI tool")
    .arg(Arg::new("input")
        .help("Input file")
        .required(true)
    )
    .arg(Arg::new("output")
        .short('o')
        .long("output")
        .help("Output file")
    )
    .arg(Arg::new("verbose")
        .short('v')
        .long("verbose")
        .help("Verbose output")
        .action(clap::ArgAction::Count)
    );
```

## Arguments

### Basic Arguments

```rust
Arg::new("input")
    .help("Input file")
    .required(true)
```

### Options

```rust
Arg::new("port")
    .short('p')
    .long("port")
    .help("Server port")
    .value_name("PORT")
    .default_value("8080")
```

### Flags

```rust
Arg::new("verbose")
    .short('v')
    .long("verbose")
    .help("Enable verbose mode")
    .action(clap::ArgAction::SetTrue)
```

### Multiple Values

```rust
Arg::new("files")
    .help("Input files")
    .num_args(1..)
    .required(true)
```

## Argument Configuration

### Validation

```rust
Arg::new("port")
    .short('p')
    .long("port")
    .value_parser(clap::value_parser!(u16).range(1024..))
```

### Groups

```rust
cmd = cmd
    .arg(Arg::new("file")
        .long("file")
        .group("input")
    )
    .arg(Arg::new("url")
        .long("url")
        .group("input")
    )
    .group(clap::ArgGroup::new("input")
        .required(true)
        .args(["file", "url"])
    );
```

## Parsing

### Get Matches

```rust
let matches = cmd.get_matches();

// Get single value
let name = matches.get_one::<String>("name").unwrap();

// Get optional value
let config = matches.get_one::<String>("config");

// Get multiple values
let files: Vec<&String> = matches.get_many::<String>("files")
    .unwrap()
    .collect();

// Get flag count
let verbose = matches.get_count("verbose");

// Check if flag is present
let force = matches.get_flag("force");
```

### Try Parse

```rust
match cmd.try_get_matches() {
    Ok(matches) => {
        // Process matches
    }
    Err(e) => {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
```

## Comparison with Derive

### Derive (Declarative)

```rust
#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    name: String,
}
```

### Builder (Programmatic)

```rust
let cmd = Command::new("myapp")
    .arg(Arg::new("name")
        .short('n')
        .long("name")
        .required(true)
    );

let matches = cmd.get_matches();
let name = matches.get_one::<String>("name").unwrap();
```

## When to Use Builder

### Use Builder When:

- CLI structure is determined at runtime
- Building CLI frameworks
- Maximum flexibility needed
- Dynamic argument addition

### Example: Dynamic CLI

```rust
fn build_cli(available_plugins: &[&str]) -> Command {
    let mut cmd = Command::new("myapp");
    
    for plugin in available_plugins {
        cmd = cmd.arg(Arg::new(plugin)
            .long(plugin)
            .help(format!("Enable {} plugin", plugin))
        );
    }
    
    cmd
}

fn main() {
    let plugins = vec!["plugin1", "plugin2", "plugin3"];
    let cmd = build_cli(&plugins);
    let matches = cmd.get_matches();
    
    // Process dynamic arguments
}
```

## Next Steps

- **[Commands](commands.md)** - Detailed command configuration
- **[Arguments](arguments.md)** - Argument configuration options
- **[Groups](groups.md)** - Argument groups