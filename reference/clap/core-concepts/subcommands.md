# Subcommands

## Overview

Subcommands allow you to create complex CLI applications with multiple operations. Think of `git` with its subcommands: `git clone`, `git commit`, `git push`.

## Basic Subcommands

### Defining Subcommands

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "myapp")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new item
    Add { name: String },
    /// Remove an item
    Remove { name: String },
    /// List all items
    List,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Add { name } => {
            println!("Adding: {}", name);
        }
        Commands::Remove { name } => {
            println!("Removing: {}", name);
        }
        Commands::List => {
            println!("Listing all items");
        }
    }
}
```

**Usage:**
```bash
$ myapp add "New Item"
Adding: New Item

$ myapp remove "Old Item"
Removing: Old Item

$ myapp list
Listing all items

$ myapp --help
Usage: myapp <COMMAND>

Commands:
  add     Add a new item
  remove  Remove an item
  list    List all items
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## Subcommand with Arguments

### Simple Arguments

```rust
#[derive(Subcommand)]
enum Commands {
    /// Add a file
    Add {
        /// File to add
        file: String,
        
        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
    },
}
```

### Complex Arguments

```rust
#[derive(Subcommand)]
enum Commands {
    /// Clone a repository
    Clone {
        /// Repository URL
        url: String,
        
        /// Local directory
        #[arg(value_name = "PATH")]
        directory: Option<String>,
        
        /// Clone recursively
        #[arg(short, long)]
        recursive: bool,
        
        /// Branch to checkout
        #[arg(short, long)]
        branch: Option<String>,
        
        /// Depth of clone
        #[arg(long, value_name = "N")]
        depth: Option<u32>,
    },
}

// Usage:
// myapp clone https://github.com/user/repo
// myapp clone https://github.com/user/repo ./local --recursive
// myapp clone https://github.com/user/repo -b main --depth 1
```

## Nested Subcommands

### Hierarchical Commands

```rust
#[derive(Subcommand)]
enum Commands {
    /// Work with configurations
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Work with items
    Item {
        #[command(subcommand)]
        command: ItemCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get configuration value
    Get { key: String },
    /// Set configuration value
    Set { key: String, value: String },
    /// List all configurations
    List,
}

#[derive(Subcommand)]
enum ItemCommands {
    /// Add item
    Add { name: String },
    /// Remove item
    Remove { name: String },
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Config { command } => match command {
            ConfigCommands::Get { key } => println!("Get: {}", key),
            ConfigCommands::Set { key, value } => println!("Set: {} = {}", key, value),
            ConfigCommands::List => println!("List configs"),
        },
        Commands::Item { command } => match command {
            ItemCommands::Add { name } => println!("Add item: {}", name),
            ItemCommands::Remove { name } => println!("Remove item: {}", name),
        },
    }
}
```

**Usage:**
```bash
$ myapp config get api.url
Get: api.url

$ myapp config set api.url https://api.example.com
Set: api.url = https://api.example.com

$ myapp item add "New Item"
Add item: New Item
```

## External Subcommand Pattern

### Dynamic Subcommands

```rust
#[derive(Subcommand)]
enum Commands {
    /// Built-in commands
    Add { name: String },
    Remove { name: String },
    
    /// External subcommand (for plugins)
    #[command(external_subcommand)]
    External(Vec<String>),
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Add { name } => println!("Adding: {}", name),
        Commands::Remove { name } => println!("Removing: {}", name),
        Commands::External(args) => {
            let plugin_name = &args[0];
            let plugin_args = &args[1..];
            println!("Running plugin: {} with args: {:?}", plugin_name, plugin_args);
        }
    }
}
```

**Usage:**
```bash
$ myapp custom-plugin --option value
Running plugin: custom-plugin with args: ["--option", "value"]
```

## Subcommand Aliases

### Short Aliases

```rust
#[derive(Subcommand)]
enum Commands {
    /// Add a file [alias: a]
    #[command(alias = "a")]
    Add {
        file: String,
    },
    /// Remove a file [alias: rm, delete]
    #[command(visible_alias = "rm")]
    #[command(alias = "delete")]
    Remove {
        file: String,
    },
}
```

**Usage:**
```bash
$ myapp a file.txt      # Using alias
$ myapp add file.txt    # Full name
$ myapp rm file.txt     # Visible alias (shown in help)
$ myapp delete file.txt # Hidden alias
```

## Global Arguments

### Arguments Available to All Subcommands

```rust
#[derive(Parser)]
struct Cli {
    /// Enable verbose output (available in all subcommands)
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Configuration file (available in all subcommands)
    #[arg(short, long, global = true)]
    config: Option<String>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add { name: String },
    Remove { name: String },
}

fn main() {
    let cli = Cli::parse();
    
    if cli.verbose {
        println!("Verbose mode enabled");
    }
    
    match cli.command {
        Commands::Add { name } => println!("Adding: {}", name),
        Commands::Remove { name } => println!("Removing: {}", name),
    }
}
```

**Usage:**
```bash
$ myapp --verbose add "Item"
Verbose mode enabled
Adding: Item

$ myapp add "Item" --verbose
Verbose mode enabled
Adding: Item
```

## Subcommand Groups

### Organizing Related Commands

```rust
#[derive(Subcommand)]
enum Commands {
    /// Database operations
    #[command(subcommand_value_name = "DB_CMD")]
    #[command(subcommand_help_heading = "Database Commands")]
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
    
    /// Cache operations
    #[command(subcommand_help_heading = "Cache Commands")]
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },
}

#[derive(Subcommand)]
enum DbCommands {
    /// Migrate database
    Migrate,
    /// Reset database
    Reset,
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Clear cache
    Clear,
    /// Show cache stats
    Stats,
}
```

## Subcommand with Struct

### Reusing Argument Structures

```rust
#[derive(Parser)]
struct AddArgs {
    /// Item name
    name: String,
    
    /// Item description
    #[arg(short, long)]
    description: Option<String>,
    
    /// Tags
    #[arg(short, long)]
    tags: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add command using struct
    Add(AddArgs),
    /// Update command using same struct
    Update(AddArgs),
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Add(args) => {
            println!("Adding: {}", args.name);
        }
        Commands::Update(args) => {
            println!("Updating: {}", args.name);
        }
    }
}
```

## Complete Example: Git-like CLI

```rust
use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "git")]
#[command(about = "A Git-like CLI tool")]
#[command(version)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone a repository
    Clone(CloneArgs),
    /// Commit changes
    Commit(CommitArgs),
    /// Push changes
    Push(PushArgs),
    /// Pull changes
    Pull,
}

#[derive(Args)]
struct CloneArgs {
    /// Repository URL
    url: String,
    
    /// Local directory name
    directory: Option<String>,
    
    /// Clone recursively
    #[arg(short, long)]
    recursive: bool,
    
    /// Branch to checkout
    #[arg(short, long)]
    branch: Option<String>,
}

#[derive(Args)]
struct CommitArgs {
    /// Commit message
    #[arg(short, long)]
    message: String,
    
    /// Stage all changes
    #[arg(short = 'a', long)]
    all: bool,
    
    /// Amend previous commit
    #[arg(long)]
    amend: bool,
}

#[derive(Args)]
struct PushArgs {
    /// Remote name
    #[arg(default_value = "origin")]
    remote: String,
    
    /// Branch name
    branch: Option<String>,
    
    /// Force push
    #[arg(short, long)]
    force: bool,
}

fn main() {
    let cli = Cli::parse();
    
    if cli.verbose {
        eprintln!("Running in verbose mode");
    }
    
    match cli.command {
        Commands::Clone(args) => {
            println!("Cloning from {}", args.url);
            if args.recursive {
                println!("  (recursively)");
            }
        }
        Commands::Commit(args) => {
            println!("Committing: {}", args.message);
            if args.all {
                println!("  (including all changes)");
            }
        }
        Commands::Push(args) => {
            println!("Pushing to {}", args.remote);
            if args.force {
                println!("  (forced)");
            }
        }
        Commands::Pull => {
            println!("Pulling changes");
        }
    }
}
```

## Best Practices

### 1. Use Descriptive Names

```rust
#[derive(Subcommand)]
enum Commands {
    /// Good: Clear what it does
    CreateUser { username: String },
    
    /// Bad: Unclear
    Cu { u: String },
}
```

### 2. Provide Help Text

```rust
#[derive(Subcommand)]
enum Commands {
    /// Deploy application to production
    /// 
    /// This command builds and deploys the application
    /// to the production environment. Requires admin access.
    Deploy {
        /// Environment to deploy to
        #[arg(short, long, default_value = "staging")]
        env: String,
    },
}
```

### 3. Group Related Commands

```rust
#[derive(Subcommand)]
enum Commands {
    /// User management
    #[command(subcommand_help_heading = "User Management")]
    User {
        #[command(subcommand)]
        command: UserCommands,
    },
    
    /// System administration
    #[command(subcommand_help_heading = "System Admin")]
    Admin {
        #[command(subcommand)]
        command: AdminCommands,
    },
}
```

### 4. Use Aliases for Common Commands

```rust
#[derive(Subcommand)]
enum Commands {
    /// List items [alias: ls]
    #[command(alias = "ls")]
    List,
    
    /// Remove item [alias: rm, delete]
    #[command(visible_alias = "rm", alias = "delete")]
    Remove { name: String },
}
```

## Next Steps

- **[Commands Structure](commands-structure.md)** - Organize command hierarchies
- **[Validation](../validation/value-validation.md)** - Validate subcommand arguments
- **[Examples](../examples/intermediate.md)** - See complete subcommand examples