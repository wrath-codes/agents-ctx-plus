# Attributes

## Overview

Attributes in the derive macro are used to customize CLI behavior. They can be applied at the struct level (command attributes) and field level (argument attributes).

## Command Attributes (Struct-Level)

### Metadata

```rust
#[derive(Parser)]
#[command(name = "myapp")]           // Binary name
#[command(bin_name = "my-app")]      // Alternative binary name
#[command(author = "Author Name")]   // Author
#[command(version = "1.0.0")]        // Version string
#[command(about = "Short desc")]     // Short description
#[command(long_about = "Long desc")] // Long description
struct Cli {
    // ...
}
```

### Help Generation

```rust
#[derive(Parser)]
#[command(
    help_template = r#"{before-help}{about-with-newline}
{usage-heading}
{tab}{usage}

{all-args}{after-help}"#,
)]
#[command(disable_help_flag = true)]         // Disable --help
#[command(disable_version_flag = true)]      // Disable --version
#[command(help_expected = true)]             // Panic if no help text
#[command(next_line_help = true)]            // Help on next line
#[command(no_binary_name = true)]            // Don't show binary name
#[command(subcommand_value_name = "CMD")]    // Subcommand value name
#[command(subcommand_help_heading = "Commands")] // Heading for subcommands
struct Cli {
    // ...
}
```

### Version Control

```rust
#[derive(Parser)]
#[command(version)]                    // Read from Cargo.toml
#[command(version = "1.0.0")]         // Manual version
#[command(long_version = "...")]      // Long version text
#[command(version_message = "...")]   // Custom version message
#[command(help_message = "...")]      // Custom help message
struct Cli {
    // ...
}
```

### Color and Formatting

```rust
#[derive(Parser)]
#[command(color = clap::ColorChoice::Auto)]   // Color: Always, Auto, Never
#[command(term_width = 80)]                    // Terminal width
#[command(max_term_width = 100)]               // Max terminal width
struct Cli {
    // ...
}
```

### Behavior

```rust
#[derive(Parser)]
#[command(dont_collapse_args_in_usage = true)]
#[command(hide_possible_values = true)]
#[command(hide_default_value = false)]
#[command(next_display_order = None)]
#[command(allow_missing_positional = true)]
#[command(args_conflicts_with_subcommands = true)]
#[command(subcommand_required = true)]
#[command(subcommand_required_else_help = true)]
#[command(subcommand_value_name = "COMMAND")]
#[command(subcommand_help_heading = "Commands")]
#[command(external_subcommand = true)]
#[command(multicall = true)]
#[command(arg_required_else_help = true)]
#[command(help_required = true)]
#[command(trailing_var_arg = true)]
struct Cli {
    // ...
}
```

## Argument Attributes (Field-Level)

### Basic Attributes

```rust
#[derive(Parser)]
struct Cli {
    #[arg(short = 'n')]              // Short flag
    #[arg(long = "name")]            // Long flag
    #[arg(visible_short_alias = 'N')] // Visible short alias
    #[arg(visible_alias = "nm")]     // Visible long alias
    #[arg(alias = "nm")]             // Hidden alias
    name: String,
    
    #[arg(value_name = "FILE")]      // Value placeholder
    #[arg(help = "Help text")]       // Short help
    #[arg(long_help = "Long help")]  // Long help
    #[arg(help_heading = "Input")]   // Group under heading
    file: String,
}
```

### Value Control

```rust
#[derive(Parser)]
struct Cli {
    #[arg(default_value = "default")]     // Default value
    #[arg(default_value_t = 10)]          // Default typed value
    #[arg(default_missing_value = "auto")] // Default when flag present
    value: String,
    
    #[arg(num_args = 1..)]               // Multiple values
    #[arg(num_args = 0..=1)]             // Range of values
    items: Vec<String>,
    
    #[arg(value_delimiter = ',')]        // Delimiter for multiple values
    list: Vec<String>,
    
    #[arg(value_terminator = "--")]      // Terminator for values
    args: Vec<String>,
}
```

### Validation

```rust
#[derive(Parser)]
struct Cli {
    #[arg(required = true)]              // Required argument
    #[arg(required_unless_present = "flag")]
    #[arg(required_if_eq_all = [("mode", "strict")])]
    name: String,
    
    #[arg(conflicts_with = "quiet")]     // Conflict with other arg
    #[arg(conflicts_with_all = ["quiet", "silent"])]
    verbose: bool,
    
    #[arg(requires = "config")]          // Requires another arg
    #[arg(requires_if("mode", "strict"))]
    debug: bool,
    
    #[arg(group = "input")]              // Argument group
    file: Option<String>,
}
```

### Environment Variables

```rust
#[derive(Parser)]
struct Cli {
    #[arg(env = "API_KEY")]              // Environment variable
    #[arg(env)]                          // Derive from field name: API_KEY
    api_key: String,
    
    #[arg(hide_env = true)]              // Hide env var from help
    #[arg(hide_env_values = true)]       // Hide env value from help
    secret: String,
}
```

### Actions

```rust
#[derive(Parser)]
struct Cli {
    #[arg(action = clap::ArgAction::SetTrue)]
    flag: bool,
    
    #[arg(action = clap::ArgAction::SetFalse)]
    no_flag: bool,
    
    #[arg(action = clap::ArgAction::Count)]
    verbose: u8,
    
    #[arg(action = clap::ArgAction::Append)]
    items: Vec<String>,
    
    #[arg(action = clap::ArgAction::Set)]
    value: String,
}
```

### Parser

```rust
#[derive(Parser)]
struct Cli {
    #[arg(value_parser = clap::value_parser!(u16))]
    port: u16,
    
    #[arg(value_parser = clap::value_parser!(u8).range(1..=100))]
    percentage: u8,
    
    #[arg(value_parser = parse_custom)]
    custom: CustomType,
}

fn parse_custom(s: &str) -> Result<CustomType, String> {
    // Custom parsing logic
}
```

## Complete Attribute Reference

### Command Attributes Table

| Attribute | Description | Example |
|-----------|-------------|---------|
| `name` | Command name | `name = "myapp"` |
| `author` | Author info | `author = "Name <email>"` |
| `version` | Version string | `version = "1.0"` |
| `about` | Short description | `about = "Does X"` |
| `long_about` | Long description | `long_about = "..."` |
| `help_template` | Help format | `help_template = "..."` |
| `color` | Color choice | `color = Auto` |
| `term_width` | Terminal width | `term_width = 80` |
| `disable_help_flag` | No --help | `disable_help_flag = true` |
| `disable_version_flag` | No --version | `disable_version_flag = true` |
| `subcommand_required` | Require subcommand | `subcommand_required = true` |
| `arg_required_else_help` | Show help if no args | `arg_required_else_help = true` |
| `multicall` | Multi-call binary | `multicall = true` |
| `trailing_var_arg` | Trailing args | `trailing_var_arg = true` |

### Argument Attributes Table

| Attribute | Description | Example |
|-----------|-------------|---------|
| `short` | Short flag | `short`, `short = 'n'` |
| `long` | Long flag | `long`, `long = "name"` |
| `visible_short_alias` | Visible short alias | `visible_short_alias = 'N'` |
| `visible_alias` | Visible long alias | `visible_alias = "nm"` |
| `alias` | Hidden alias | `alias = "nm"` |
| `value_name` | Value placeholder | `value_name = "FILE"` |
| `help` | Help text | `help = "Description"` |
| `long_help` | Long help | `long_help = "..."` |
| `help_heading` | Group heading | `help_heading = "Input"` |
| `default_value` | Default | `default_value = "x"` |
| `default_value_t` | Typed default | `default_value_t = 10` |
| `num_args` | Number of values | `num_args = 1..` |
| `value_delimiter` | Delimiter | `value_delimiter = ','` |
| `required` | Required | `required = true` |
| `conflicts_with` | Conflicts | `conflicts_with = "x"` |
| `requires` | Requires | `requires = "x"` |
| `group` | Group | `group = "input"` |
| `env` | Environment var | `env = "VAR"` |
| `hide_env` | Hide env | `hide_env = true` |
| `action` | Action | `action = ArgAction::Count` |
| `value_parser` | Value parser | `value_parser = ...` |
| `hide` | Hide from help | `hide = true` |
| `global` | Global arg | `global = true` |

## Examples

### Complex CLI

```rust
#[derive(Parser)]
#[command(
    name = "app",
    version,
    about = "A complex application",
    long_about = "A complex application that demonstrates various features.

This is a longer description that appears with --help.
It can span multiple lines and paragraphs.",
    color = clap::ColorChoice::Auto,
)]
struct Cli {
    /// Configuration file
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Path to configuration file",
        long_help = "Path to the configuration file.

If not provided, the application will search for:
- ./config.toml
- ~/.config/app/config.toml
- /etc/app/config.toml",
        env = "APP_CONFIG",
        global = true,
    )]
    config: Option<String>,
    
    /// Verbose output
    #[arg(
        short,
        long,
        action = clap::ArgAction::Count,
        help = "Increase verbosity",
        global = true,
    )]
    verbose: u8,
    
    #[command(subcommand)]
    command: Commands,
}
```

## Next Steps

- **[Field Attributes](field-attributes.md)** - Detailed field attribute guide
- **[Advanced Derive](advanced-derive.md)** - Complex derive patterns